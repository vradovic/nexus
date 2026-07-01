import { AfterViewInit, Component, ViewChild, effect, input, output, signal } from '@angular/core';
import {
    MoveChange,
    NgxChessBoardComponent as NgxChessBoardViewComponent,
    NgxChessBoardModule,
    NgxChessBoardService,
} from 'ngx-chess-board';
import { ChessColor, ChessMove, ChessPosition } from '../../services/realtime.service';

const FILES = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'] as const;
const RANKS = [8, 7, 6, 5, 4, 3, 2, 1] as const;
const PIECE_TO_FEN: Record<string, string> = {
    wK: 'K',
    wQ: 'Q',
    wR: 'R',
    wB: 'B',
    wN: 'N',
    wP: 'P',
    bK: 'k',
    bQ: 'q',
    bR: 'r',
    bB: 'b',
    bN: 'n',
    bP: 'p',
};
const FEN_TO_PIECE = Object.fromEntries(
    Object.entries(PIECE_TO_FEN).map(([piece, fen]) => [fen, piece]),
) as Record<string, string>;

@Component({
    selector: 'app-chess-board',
    imports: [NgxChessBoardModule],
    providers: [NgxChessBoardService],
    templateUrl: './chess-board.component.html',
    styleUrl: './chess-board.component.scss',
})
export class ChessBoardComponent implements AfterViewInit {
    readonly position = input<ChessPosition | null>(null);
    readonly turn = input<ChessColor>('w');
    readonly orientation = input<ChessColor>('w');
    readonly disabled = input(false);
    readonly move = output<ChessMove>();

    @ViewChild('board') private board?: NgxChessBoardViewComponent;

    readonly awaitingServerMove = signal(false);
    private applyingServerState = false;
    private boardReady = false;
    private displayedFen = '';
    private displayedOrientation: ChessColor = 'w';
    private serverPosition: ChessPosition = {};

    constructor() {
        effect(() => {
            const position = this.position() ?? {};
            const turn = this.turn();
            const orientation = this.orientation();

            if (this.boardReady) {
                this.applyServerState(position, turn, orientation);
            }
        });
    }

    ngAfterViewInit() {
        this.boardReady = true;
        this.applyServerState(this.position() ?? {}, this.turn(), this.orientation());
    }

    onMoveChange(change: MoveChange) {
        if (this.applyingServerState) {
            return;
        }

        const nextPosition = positionFromFen(change.fen);
        const inferredMove = inferMove(this.serverPosition, nextPosition);

        if (!inferredMove) {
            this.displayedFen = change.fen;
            this.applyServerState(this.serverPosition, this.turn(), this.orientation());
            return;
        }

        this.displayedFen = change.fen;
        this.awaitingServerMove.set(true);
        this.move.emit(inferredMove);
    }

    private applyServerState(position: ChessPosition, turn: ChessColor, orientation: ChessColor) {
        if (!this.board) {
            return;
        }

        const fen = positionToFen(position, turn);
        this.serverPosition = { ...position };
        this.awaitingServerMove.set(false);

        this.applyingServerState = true;
        try {
            if (fen !== this.displayedFen) {
                this.board.setFEN(fen);
                this.displayedFen = fen;
                this.displayedOrientation = 'w';
            }

            if (orientation !== this.displayedOrientation) {
                this.board.reverse();
                this.displayedOrientation = orientation;
            }
        } finally {
            queueMicrotask(() => {
                this.applyingServerState = false;
            });
        }
    }
}

function positionToFen(position: ChessPosition, turn: ChessColor) {
    const rows = RANKS.map(rank => {
        let emptySquares = 0;
        let row = '';

        for (const file of FILES) {
            const piece = position[`${file}${rank}`] ?? '';

            if (!piece) {
                emptySquares += 1;
                continue;
            }

            if (emptySquares) {
                row += String(emptySquares);
                emptySquares = 0;
            }

            row += PIECE_TO_FEN[piece] ?? '';
        }

        return row + (emptySquares ? String(emptySquares) : '');
    });

    return `${rows.join('/')} ${turn} - - 0 1`;
}

function positionFromFen(fen: string): ChessPosition {
    const [board] = fen.split(' ');
    const position: ChessPosition = {};

    board.split('/').forEach((row, rankIndex) => {
        let fileIndex = 0;
        const rank = 8 - rankIndex;

        for (const token of row) {
            const emptySquares = Number(token);

            if (Number.isInteger(emptySquares) && emptySquares > 0) {
                fileIndex += emptySquares;
                continue;
            }

            const file = FILES[fileIndex];
            const piece = FEN_TO_PIECE[token];

            if (file && piece) {
                position[`${file}${rank}`] = piece;
            }

            fileIndex += 1;
        }
    });

    return position;
}

function inferMove(previous: ChessPosition, next: ChessPosition): ChessMove | null {
    const changedSquares = allSquares().filter(square => (previous[square] ?? '') !== (next[square] ?? ''));
    const sourceCandidates = changedSquares.filter(square => previous[square] && previous[square] !== next[square]);
    const targetCandidates = changedSquares.filter(square => next[square] && previous[square] !== next[square]);

    const castleKingSource = sourceCandidates.find(square => previous[square]?.endsWith('K'));
    const castleKingTarget = targetCandidates.find(square => next[square]?.endsWith('K'));

    if (castleKingSource && castleKingTarget) {
        return {
            source: castleKingSource,
            target: castleKingTarget,
            piece: previous[castleKingSource],
        };
    }

    for (const source of sourceCandidates) {
        const movingPiece = previous[source];
        const target = targetCandidates.find(candidate => next[candidate] === movingPiece);

        if (target) {
            return { source, target, piece: movingPiece };
        }
    }

    if (sourceCandidates.length === 1 && targetCandidates.length === 1) {
        const source = sourceCandidates[0];

        return {
            source,
            target: targetCandidates[0],
            piece: previous[source],
        };
    }

    const promotionSource = sourceCandidates.find(square => previous[square]?.endsWith('P'));
    const promotionTarget = targetCandidates.find(square => sameColor(previous[promotionSource ?? ''], next[square]));

    if (promotionSource && promotionTarget) {
        return {
            source: promotionSource,
            target: promotionTarget,
            piece: previous[promotionSource],
        };
    }

    return null;
}

function sameColor(a = '', b = '') {
    return !!a && !!b && a[0] === b[0];
}

function allSquares() {
    return RANKS.flatMap(rank => FILES.map(file => `${file}${rank}`));
}
