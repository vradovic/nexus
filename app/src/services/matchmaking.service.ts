import { HttpClient } from '@angular/common/http';
import { DestroyRef, computed, inject, Injectable, signal } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import { environment } from '../environments/environment';
import {
    CreateMatchmakingRuleRequest,
    MatchmakingRule,
    MatchmakingStatus,
    MatchmakingTicket,
    PendingMatch,
    UpdateMatchmakingRuleEnabled,
} from '../models/matchmaking.model';

const POLL_INTERVAL_MS = 1500;

@Injectable({ providedIn: 'root' })
export class MatchmakingService {
    private readonly destroyRef = inject(DestroyRef);
    private readonly http = inject(HttpClient);
    private pollTimer: number | null = null;
    private readonly confirmedMatchIds = new Set<string>();
    private readonly confirmingMatchId = signal('');

    readonly rules = signal<MatchmakingRule[]>([]);
    readonly status = signal<MatchmakingStatus>({ ticket: null, pending_match: null });
    readonly busy = signal(false);
    readonly loadingRules = signal(false);
    readonly message = signal('Select a queue');
    readonly error = signal<string | null>(null);
    readonly confirming = computed(() => !!this.confirmingMatchId());
    readonly activeTicketKey = computed(() => {
        const status = this.status();

        return status.pending_match?.ticket_key ?? status.ticket?.ticket_key ?? '';
    });

    constructor() {
        this.destroyRef.onDestroy(() => this.stopPolling());
    }

    async loadRules() {
        this.loadingRules.set(true);
        this.error.set(null);

        try {
            const rules = await firstValueFrom(
                this.http.get<MatchmakingRule[]>(`${environment.matchmakingApiUrl}/matchmaking/rules`),
            );
            this.rules.set(rules);
        } catch (error) {
            this.captureError(error, 'Failed to load matchmaking rules.');
        } finally {
            this.loadingRules.set(false);
        }
    }

    async joinQueue(ticketKey: string) {
        if (!ticketKey || this.busy()) {
            return;
        }

        this.busy.set(true);
        this.message.set(`Joining ${ticketKey}`);
        this.error.set(null);

        try {
            const ticket = await firstValueFrom(
                this.http.post<MatchmakingTicket>(`${environment.matchmakingApiUrl}/join`, {
                    ticket_key: ticketKey,
                }),
            );
            this.status.set({ ticket, pending_match: null });
            this.message.set(`Searching ${ticket.ticket_key}`);
            this.startPolling();
        } catch (error) {
            this.captureError(error, 'Failed to join queue.');
            await this.refreshStatus();
        } finally {
            this.busy.set(false);
        }
    }

    async leaveQueue() {
        if (this.busy() || !this.status().ticket) {
            return;
        }

        this.busy.set(true);
        this.message.set('Leaving queue');
        this.error.set(null);

        try {
            await firstValueFrom(this.http.post<void>(`${environment.matchmakingApiUrl}/leave`, {}));
            this.confirmedMatchIds.clear();
            this.status.set({ ticket: null, pending_match: null });
            this.message.set('Select a queue');
        } catch (error) {
            this.captureError(error, 'Failed to leave queue.');
        } finally {
            this.busy.set(false);
        }
    }

    async refreshStatus() {
        try {
            const status = await firstValueFrom(
                this.http.get<MatchmakingStatus>(`${environment.matchmakingApiUrl}/status`),
            );
            this.status.set(status);
            this.syncMessage(status);
        } catch (error) {
            this.captureError(error, 'Failed to refresh matchmaking.');
        }
    }

    async confirmPendingMatch(match: PendingMatch) {
        if (this.confirmingMatchId() === match.id || this.confirmedMatchIds.has(match.id)) {
            return;
        }

        this.confirmingMatchId.set(match.id);
        this.message.set('Match found. Confirming');
        this.error.set(null);

        try {
            await firstValueFrom(
                this.http.post<void>(`${environment.matchmakingApiUrl}/matches/${match.id}/confirm`, {}),
            );
            this.confirmedMatchIds.add(match.id);
            this.message.set('Match confirmed. Opening channel');
        } catch (error) {
            this.confirmedMatchIds.delete(match.id);
            this.captureError(error, 'Failed to confirm match.');
        } finally {
            this.confirmingMatchId.set('');
        }
    }

    async declinePendingMatch(match: PendingMatch) {
        await firstValueFrom(
            this.http.post<void>(`${environment.matchmakingApiUrl}/matches/${match.id}/decline`, {}),
        );
        this.confirmedMatchIds.delete(match.id);
        await this.refreshStatus();
    }

    startPolling() {
        if (this.pollTimer !== null) {
            return;
        }

        this.pollTimer = window.setInterval(() => {
            void this.refreshStatus();
        }, POLL_INTERVAL_MS);
    }

    stopPolling() {
        if (this.pollTimer === null) {
            return;
        }

        window.clearInterval(this.pollTimer);
        this.pollTimer = null;
    }

    async adminRules() {
        return firstValueFrom(
            this.http.get<MatchmakingRule[]>(`${environment.matchmakingApiUrl}/admin/matchmaking/rules`),
        );
    }

    async createAdminRule(payload: CreateMatchmakingRuleRequest) {
        return firstValueFrom(
            this.http.post<MatchmakingRule>(
                `${environment.matchmakingApiUrl}/admin/matchmaking/rules`,
                payload,
            ),
        );
    }

    async updateAdminRules(rules: UpdateMatchmakingRuleEnabled[]) {
        return firstValueFrom(
            this.http.patch<MatchmakingRule[]>(
                `${environment.matchmakingApiUrl}/admin/matchmaking/rules/enabled`,
                { rules },
            ),
        );
    }

    reset() {
        this.confirmedMatchIds.clear();
        this.confirmingMatchId.set('');
        this.status.set({ ticket: null, pending_match: null });
        this.message.set('Select a queue');
        this.error.set(null);
        this.stopPolling();
    }

    private syncMessage(status: MatchmakingStatus) {
        if (this.busy()) {
            return;
        }

        if (status.pending_match) {
            this.message.set(
                this.confirmedMatchIds.has(status.pending_match.id)
                    ? 'Match confirmed. Opening channel'
                    : 'Match found. Confirming',
            );
        } else if (status.ticket) {
            this.message.set(`Searching ${status.ticket.ticket_key}`);
        } else {
            this.message.set('Select a queue');
        }
    }

    private captureError(error: unknown, fallback: string) {
        const message = error instanceof Error ? error.message : fallback;
        this.error.set(message);
        this.message.set(message || fallback);
    }
}
