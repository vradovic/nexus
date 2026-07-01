import { DestroyRef, effect, inject, Injectable, signal } from '@angular/core';
import { Subscription } from 'rxjs';
import { webSocket, WebSocketSubject } from 'rxjs/webSocket';
import { AuthService } from './auth.service';
import { environment } from '../environments/environment';

export type RealtimeConnectionStatus =
    | 'disconnected'
    | 'connecting'
    | 'connected'
    | 'error';

export type ChessColor = 'w' | 'b';
export type ChessPosition = Record<string, string>;

export interface ChessMove {
    source: string;
    target: string;
    piece: string;
    promotion?: string;
}

export interface ActiveMatch {
    matchId: string;
    channel: string;
    ticketKey: string;
    playerIds: string[];
}

export interface MatchFoundMessage {
    type: 'match.found';
    matchId?: string;
    match_id?: string;
    channel?: string;
    ticketKey?: string;
    ticket_key?: string;
    playerIds?: string[];
    player_ids?: string[];
    position?: ChessPosition;
    turn?: ChessColor;
    moves?: number;
}

export interface ChessPositionMessage {
    type: 'chess.position';
    action:
        | 'move'
        | 'reset'
        | 'rejected'
        | 'sync_response';
    clientId?: string;
    clientName?: string;
    matchId?: string;
    match_id?: string;
    channel?: string;
    player_ids?: string[];
    position?: ChessPosition;
    turn?: ChessColor;
    moves?: number;
    status?: string;
    check?: boolean;
    match_over?: boolean;
    winner?: ChessColor | '';
    end_reason?: string;
    move?: ChessMove;
    error?: string;
}

export interface ChessSyncRequestMessage {
    type: 'chess.sync_request';
    clientId?: string;
    clientName?: string;
    matchId?: string;
    channel?: string;
    createdAt?: number;
}

export interface ChatMessagePayload {
    id?: string;
    channel: string;
    sender_id?: string;
    body: string;
    created_at?: string;
}

export interface ChatMessage {
    type: 'chat.message';
    channel?: string;
    message?: ChatMessagePayload;
}

export interface UnknownRealtimeMessage {
    type: 'unknown';
    raw: string;
}

export type IncomingRealtimeMessage =
    | MatchFoundMessage
    | ChessPositionMessage
    | ChessSyncRequestMessage
    | ChatMessage
    | UnknownRealtimeMessage;

export type OutgoingRealtimeMessage =
    | ChessPositionMessage
    | ChessSyncRequestMessage;

type SocketMessage = IncomingRealtimeMessage | OutgoingRealtimeMessage;

@Injectable({ providedIn: 'root' })
export class RealtimeService {
    private readonly authService = inject(AuthService);
    private readonly destroyRef = inject(DestroyRef);

    private socket: WebSocketSubject<SocketMessage> | null = null;
    private socketSubscription: Subscription | null = null;

    readonly status = signal<RealtimeConnectionStatus>('disconnected');
    readonly lastMessage = signal<IncomingRealtimeMessage | null>(null);
    readonly lastEventType = signal<string>('none');
    readonly activeMatch = signal<ActiveMatch | null>(null);
    readonly currentPosition = signal<ChessPosition | null>(null);
    readonly turn = signal<ChessColor>('w');
    readonly moves = signal(0);
    readonly error = signal<string | null>(null);

    constructor() {
        effect(() => {
            const token = this.authService.accessToken();

            if (!token) {
                this.disconnect();
                this.resetGameState();
                return;
            }

            this.connect(token);
        });

        this.destroyRef.onDestroy(() => this.disconnect());
    }

    sendMove(move: ChessMove, clientName: string, position?: ChessPosition) {
        return this.sendBoardPosition({
            action: 'move',
            clientName,
            move,
            position,
        });
    }

    sendReset(clientName: string) {
        return this.sendBoardPosition({
            action: 'reset',
            clientName,
            move: undefined,
        });
    }

    sendSyncRequest(clientName: string) {
        const activeMatch = this.activeMatch();

        if (!activeMatch) {
            return false;
        }

        return this.send({
            type: 'chess.sync_request',
            clientName,
            matchId: activeMatch.matchId,
            channel: activeMatch.channel,
            createdAt: Date.now(),
        });
    }

    send(message: OutgoingRealtimeMessage) {
        if (!this.socket || this.status() !== 'connected') {
            this.error.set('socket is not connected');
            return false;
        }

        this.socket.next(message);
        return true;
    }

    disconnect() {
        this.socketSubscription?.unsubscribe();
        this.socketSubscription = null;

        this.socket?.complete();
        this.socket = null;

        this.status.set('disconnected');
    }

    private connect(token: string) {
        if (this.socket && this.status() === 'connected') {
            return;
        }

        this.disconnect();
        this.error.set(null);
        this.status.set('connecting');

        this.socket = webSocket<SocketMessage>({
            url: this.websocketUrl(token),
            binaryType: 'arraybuffer',
            serializer: message => JSON.stringify(message),
            deserializer: event => decodeMessage(event.data),
            openObserver: {
                next: () => {
                    this.error.set(null);
                    this.status.set('connected');
                },
            },
            closeObserver: {
                next: () => {
                    this.status.set('disconnected');
                    this.socket = null;
                    this.socketSubscription = null;
                },
            },
        });

        this.socketSubscription = this.socket.subscribe({
            next: message => this.applyMessage(message as IncomingRealtimeMessage),
            error: () => {
                this.status.set('error');
                this.error.set('realtime connection error');
                this.socket = null;
                this.socketSubscription = null;
            },
            complete: () => {
                this.status.set('disconnected');
                this.socket = null;
                this.socketSubscription = null;
            },
        });
    }

    private websocketUrl(token: string) {
        return `${environment.realtimeWsUrl}/ws?token=${encodeURIComponent(token)}`;
    }

    private sendBoardPosition(options: {
        action: ChessPositionMessage['action'];
        clientName: string;
        move?: ChessMove;
        position?: ChessPosition;
    }) {
        const activeMatch = this.activeMatch();

        if (!activeMatch) {
            this.error.set('join a match before sending chess moves');
            return false;
        }

        return this.send({
            type: 'chess.position',
            action: options.action,
            clientName: options.clientName,
            matchId: activeMatch.matchId,
            channel: activeMatch.channel,
            move: options.move,
            moves: this.moves(),
            position: options.position ?? this.currentPosition() ?? undefined,
        });
    }

    private applyMessage(message: IncomingRealtimeMessage) {
        this.lastMessage.set(message);
        this.lastEventType.set(message.type);

        switch (message.type) {
            case 'match.found':
                this.enterMatch(message);
                break;
            case 'chess.position':
                this.applyChessPosition(message);
                break;
            case 'chess.sync_request':
            case 'chat.message':
            case 'unknown':
                break;
        }
    }

    private enterMatch(message: MatchFoundMessage) {
        const match = normalizeMatchMessage(message);

        if (!match) {
            this.error.set('match notification was missing channel details');
            return;
        }

        this.activeMatch.set(match);
        this.currentPosition.set(message.position ?? null);
        this.turn.set(message.turn ?? 'w');
        this.moves.set(message.moves ?? 0);
    }

    private applyChessPosition(message: ChessPositionMessage) {
        if (message.position) {
            this.currentPosition.set(message.position);
        }

        if (typeof message.moves === 'number' && Number.isFinite(message.moves)) {
            this.moves.set(message.moves);
        }

        if (message.turn) {
            this.turn.set(message.turn);
        }

        if (message.error) {
            this.error.set(message.error);
        }

        if (message.match_over) {
            this.activeMatch.set(null);
        }
    }

    private resetGameState() {
        this.lastMessage.set(null);
        this.lastEventType.set('none');
        this.activeMatch.set(null);
        this.currentPosition.set(null);
        this.turn.set('w');
        this.moves.set(0);
        this.error.set(null);
    }
}

function normalizeMatchMessage(message: MatchFoundMessage): ActiveMatch | null {
    const matchId = message.matchId ?? message.match_id ?? '';
    const channel = message.channel ?? '';

    if (!matchId || !channel) {
        return null;
    }

    return {
        matchId,
        channel,
        ticketKey: message.ticketKey ?? message.ticket_key ?? '',
        playerIds: message.playerIds ?? message.player_ids ?? [],
    };
}

function decodeMessage(data: string | ArrayBuffer | Blob): IncomingRealtimeMessage {
    if (typeof data === 'string') {
        return parseMessage(data);
    }

    if (data instanceof ArrayBuffer) {
        return parseMessage(new TextDecoder().decode(data));
    }

    return {
        type: 'unknown',
        raw: '[unsupported blob websocket payload]',
    };
}

function parseMessage(text: string): IncomingRealtimeMessage {
    try {
        const message = JSON.parse(text) as IncomingRealtimeMessage;

        if (!message || typeof message !== 'object' || !('type' in message)) {
            return { type: 'unknown', raw: text };
        }

        return message;
    } catch {
        return { type: 'unknown', raw: text };
    }
}
