import { Component, OnInit, computed, effect, inject } from '@angular/core';
import { Router } from '@angular/router';
import { MatchmakingService } from '../../services/matchmaking.service';
import { RealtimeService } from '../../services/realtime.service';

@Component({
    selector: 'app-lobby',
    imports: [],
    templateUrl: './lobby.component.html',
    styleUrl: './lobby.component.scss',
})
export class LobbyComponent implements OnInit {
    readonly matchmaking = inject(MatchmakingService);
    readonly realtime = inject(RealtimeService);
    private readonly router = inject(Router);

    readonly status = this.matchmaking.status;
    readonly activeMatch = this.realtime.activeMatch;
    readonly expiresIn = computed(() => {
        const expires = this.status().pending_match?.expires_at_unix_seconds;

        if (!expires) {
            return 'none';
        }

        return `${Math.max(0, expires - Math.floor(Date.now() / 1000))}s`;
    });

    constructor() {
        effect(() => {
            const match = this.realtime.activeMatch();

            if (match) {
                this.matchmaking.stopPolling();
                void this.router.navigate(['/game']);
            }
        });

        effect(() => {
            const pending = this.matchmaking.status().pending_match;
            const connected = this.realtime.status() === 'connected';

            if (pending && connected) {
                void this.matchmaking.confirmPendingMatch(pending);
            }
        });
    }

    ngOnInit() {
        void this.matchmaking.loadRules();
        void this.matchmaking.refreshStatus();
        this.matchmaking.startPolling();
    }

    join(ticketKey: string) {
        void this.matchmaking.joinQueue(ticketKey);
    }

    leave() {
        void this.matchmaking.leaveQueue();
    }

    shortId(id?: string) {
        return id ? id.slice(0, 8) : 'none';
    }
}
