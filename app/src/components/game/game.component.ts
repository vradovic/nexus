import { Component, computed, effect, inject, OnInit, signal } from '@angular/core';
import { Router } from '@angular/router';
import { firstValueFrom } from 'rxjs';
import { ChessBoardComponent } from '../chess-board/chess-board.component';
import { ChatMessage as SavedChatMessage } from '../../models/social.model';
import { AuthService } from '../../services/auth.service';
import {
    ChatMessage as RealtimeChatMessage,
    ChessMove,
    ChessPositionMessage,
    IncomingRealtimeMessage,
    RealtimeService,
} from '../../services/realtime.service';
import { SocialService } from '../../services/social.service';

@Component({
    selector: 'app-game',
    imports: [ChessBoardComponent],
    templateUrl: './game.component.html',
    styleUrl: './game.component.scss',
})
export class GameComponent implements OnInit {
    readonly realtime = inject(RealtimeService);
    readonly auth = inject(AuthService);
    private readonly social = inject(SocialService);
    private readonly router = inject(Router);

    readonly flipped = signal(false);
    readonly eventLog = signal<string[]>([]);
    readonly chatDraft = signal('');
    readonly chatMessages = signal<SavedChatMessage[]>([]);
    readonly chatBusy = signal(false);
    readonly chatStatus = signal('');
    readonly sentFriendRequestIds = signal<Set<string>>(new Set());
    readonly blockedUserIds = signal<Set<string>>(new Set());
    readonly friendStatus = signal('');
    readonly blockStatus = signal('');
    private readonly seenChatMessageIds = new Set<string>();
    private lastRealtimeMessage: IncomingRealtimeMessage | null = null;

    readonly currentPosition = computed(() => this.realtime.currentPosition() ?? {});
    readonly match = this.realtime.activeMatch;
    readonly playerColor = computed<'w' | 'b'>(() => {
        const userId = this.auth.currentUser()?.id;
        const match = this.match();

        return match?.playerIds[1] === userId ? 'b' : 'w';
    });
    readonly boardOrientation = computed<'w' | 'b'>(() => {
        const base = this.playerColor();

        return this.flipped() ? opposite(base) : base;
    });
    readonly opponentId = computed(() => {
        const userId = this.auth.currentUser()?.id;
        const players = this.match()?.playerIds ?? [];

        return players.find(playerId => playerId !== userId) ?? '';
    });
    readonly canSendFriendRequest = computed(() => {
        const opponentId = this.opponentId();

        return !!opponentId &&
            !this.sentFriendRequestIds().has(opponentId) &&
            !this.blockedUserIds().has(opponentId);
    });
    readonly canBlockOpponent = computed(() => {
        const opponentId = this.opponentId();

        return !!opponentId && !this.blockedUserIds().has(opponentId);
    });

    constructor() {
        effect(() => {
            const match = this.match();

            if (!match) {
                return;
            }

            void this.loadChat(match.channel);
            void this.loadGameSocialState();
        });

        effect(() => {
            const message = this.realtime.lastMessage();

            if (!message || message === this.lastRealtimeMessage) {
                return;
            }

            this.lastRealtimeMessage = message;
            this.applyRealtimeMessage(message);
        });
    }

    ngOnInit() {
        if (!this.match()) {
            void this.router.navigate(['/']);
            return;
        }

        this.writeLog(`match found: ${this.match()?.ticketKey || 'duel'}`);
        this.realtime.sendSyncRequest(this.auth.displayName());
    }

    move(move: ChessMove) {
        if (!this.match()) {
            this.writeLog('join a match before moving pieces');
            return;
        }

        if (this.realtime.status() !== 'connected') {
            this.writeLog('connect before moving pieces');
            return;
        }

        this.realtime.sendMove(move, this.auth.displayName());
    }

    resetBoard() {
        this.realtime.sendReset(this.auth.displayName());
    }

    flipBoard() {
        this.flipped.update(value => !value);
    }

    updateChatDraft(event: Event) {
        this.chatDraft.set((event.target as HTMLInputElement).value);
    }

    async sendChat(event: Event) {
        event.preventDefault();

        const match = this.match();
        const user = this.auth.currentUser();
        const body = this.chatDraft().trim();

        if (!match || !user || !body || this.chatBusy()) {
            return;
        }

        this.chatBusy.set(true);
        this.chatStatus.set('Sending');

        try {
            const message = await firstValueFrom(this.social.sendChatMessage(match.channel, user.id, body));
            this.chatDraft.set('');
            this.appendChatMessage(message);
            this.chatStatus.set('');
        } catch {
            this.chatStatus.set('Failed to send message.');
        } finally {
            this.chatBusy.set(false);
        }
    }

    async sendFriendRequest() {
        const opponentId = this.opponentId();

        if (!opponentId || !this.canSendFriendRequest()) {
            return;
        }

        this.friendStatus.set('Sending');

        try {
            await firstValueFrom(this.social.sendFriendRequest(opponentId));
            this.sentFriendRequestIds.update(ids => new Set(ids).add(opponentId));
            this.friendStatus.set('Friend request sent');
        } catch {
            this.friendStatus.set('Failed to send friend request.');
        }
    }

    async blockOpponent() {
        const opponentId = this.opponentId();

        if (!opponentId || !this.canBlockOpponent()) {
            return;
        }

        this.blockStatus.set('Blocking');

        try {
            const blocked = await firstValueFrom(this.social.blockUser(opponentId));
            this.blockedUserIds.update(ids => new Set(ids).add(blocked.blocked_user_id));
            this.friendStatus.set('');
            this.blockStatus.set('Player blocked');
        } catch {
            this.blockStatus.set('Failed to block player.');
        }
    }

    shortId(id?: string) {
        return id ? id.slice(0, 8) : 'none';
    }

    senderLabel(senderId: string) {
        return senderId === this.auth.currentUser()?.id ? 'You' : this.shortId(senderId);
    }

    chatTime(value: string) {
        const date = new Date(value);

        if (Number.isNaN(date.getTime())) {
            return '';
        }

        return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    private async loadChat(channel: string) {
        this.chatStatus.set('Loading');

        try {
            const messages = await firstValueFrom(this.social.chatMessages(channel));

            if (this.match()?.channel !== channel) {
                return;
            }

            this.seenChatMessageIds.clear();
            this.chatMessages.set([]);
            for (const message of messages) {
                this.appendChatMessage(message);
            }
            this.chatStatus.set('');
        } catch {
            this.chatStatus.set('Failed to load chat.');
        }
    }

    private async loadGameSocialState() {
        try {
            const [requests, blocks] = await Promise.all([
                firstValueFrom(this.social.friendRequests()),
                firstValueFrom(this.social.blocks()),
            ]);
            this.sentFriendRequestIds.set(new Set(requests.outgoing.map(request => request.recipient_id)));
            this.blockedUserIds.set(new Set(blocks.map(blocked => blocked.blocked_user_id)));
        } catch {
            this.friendStatus.set('');
        }
    }

    private appendChatMessage(message: SavedChatMessage) {
        if (message.id && this.seenChatMessageIds.has(message.id)) {
            return;
        }

        if (message.id) {
            this.seenChatMessageIds.add(message.id);
        }

        this.chatMessages.update(messages => [...messages, message]);
    }

    private applyRealtimeMessage(message: IncomingRealtimeMessage) {
        if (message.type === 'chat.message') {
            this.applyChatMessage(message);
            return;
        }

        if (message.type === 'chess.position') {
            this.writeMoveLog(message);

            if (message.match_over) {
                void this.router.navigate(['/']);
            }
        }
    }

    private applyChatMessage(message: RealtimeChatMessage) {
        const payload = message.message;
        const channel = message.channel ?? payload?.channel;

        if (!payload || !this.match()?.channel || channel !== this.match()?.channel) {
            return;
        }

        this.appendChatMessage({
            id: payload.id ?? '',
            channel: payload.channel,
            sender_id: payload.sender_id ?? '',
            body: payload.body,
            created_at: payload.created_at ?? new Date().toISOString(),
        });
    }

    private writeMoveLog(message: ChessPositionMessage) {
        if (message.match_over) {
            this.writeLog(matchOverSummary(message));
            return;
        }

        if (message.action === 'rejected') {
            this.writeLog(message.error || 'move rejected');
            return;
        }

        if (message.action === 'reset') {
            this.writeLog(`${message.clientName || 'player'} reset the board`);
            return;
        }

        if (message.action === 'sync_response') {
            this.writeLog(`synced from ${message.clientName || 'player'}`);
            return;
        }

        if (message.move) {
            this.writeLog(`${message.clientName || 'player'} moved ${message.move.piece} ${message.move.source}-${message.move.target}`);
            return;
        }

        this.writeLog(`${message.clientName || 'player'} updated the board`);
    }

    private writeLog(message: string) {
        this.eventLog.update(items => [message, ...items].slice(0, 16));
    }
}

function opposite(color: 'w' | 'b'): 'w' | 'b' {
    return color === 'w' ? 'b' : 'w';
}

function matchOverSummary(payload: ChessPositionMessage) {
    if (payload.end_reason === 'checkmate') {
        return `Checkmate. ${winnerLabel(payload.winner)} wins.`;
    }

    if (payload.end_reason === 'stalemate') {
        return 'Stalemate. Match over.';
    }

    return 'Match over.';
}

function winnerLabel(winner?: 'w' | 'b' | '') {
    if (winner === 'w') {
        return 'White';
    }

    if (winner === 'b') {
        return 'Black';
    }

    return 'No one';
}
