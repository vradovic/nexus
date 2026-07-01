import { Component, OnInit, computed, inject, signal } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import { MatchmakingRule } from '../../models/matchmaking.model';
import { ActiveUserProfile, ChatMessage } from '../../models/social.model';
import { MatchmakingService } from '../../services/matchmaking.service';
import { SocialService } from '../../services/social.service';

type AdminTab = 'users' | 'chats' | 'rules';

@Component({
    selector: 'app-admin',
    imports: [],
    templateUrl: './admin.component.html',
    styleUrl: './admin.component.scss',
})
export class AdminComponent implements OnInit {
    private readonly social = inject(SocialService);
    private readonly matchmaking = inject(MatchmakingService);

    readonly tab = signal<AdminTab>('users');
    readonly status = signal('Ready');
    readonly busy = signal(false);
    readonly search = signal('');
    readonly activeUsers = signal<ActiveUserProfile[]>([]);
    readonly chatMessages = signal<ChatMessage[]>([]);
    readonly rules = signal<MatchmakingRule[]>([]);
    readonly changedRules = signal<Map<string, boolean>>(new Map());
    readonly newTicketKey = signal('');
    readonly newRequiredPlayers = signal(2);
    readonly filteredUsers = computed(() => {
        const query = this.search().trim().toLowerCase();

        if (!query) {
            return this.activeUsers();
        }

        return this.activeUsers().filter(user =>
            [user.id, user.first_name, user.last_name, this.profileName(user.first_name, user.last_name, user.id)]
                .join(' ')
                .toLowerCase()
                .includes(query),
        );
    });

    ngOnInit() {
        void this.loadCurrentTab();
    }

    selectTab(tab: AdminTab) {
        this.tab.set(tab);
        void this.loadCurrentTab();
    }

    updateSearch(event: Event) {
        this.search.set((event.target as HTMLInputElement).value);
    }

    updateNewTicketKey(event: Event) {
        this.newTicketKey.set((event.target as HTMLInputElement).value);
    }

    updateNewRequiredPlayers(event: Event) {
        this.newRequiredPlayers.set(Number((event.target as HTMLInputElement).value));
    }

    async loadCurrentTab() {
        if (this.busy()) {
            return;
        }

        this.busy.set(true);
        this.status.set('Loading');

        try {
            if (this.tab() === 'users') {
                this.activeUsers.set(await firstValueFrom(this.social.activeUsers()));
            } else if (this.tab() === 'chats') {
                this.chatMessages.set(await firstValueFrom(this.social.adminChatMessages()));
            } else {
                this.rules.set(await this.matchmaking.adminRules());
                this.changedRules.set(new Map());
            }
            this.status.set('Ready');
        } catch {
            this.status.set('Failed to load admin data.');
        } finally {
            this.busy.set(false);
        }
    }

    toggleRule(rule: MatchmakingRule, event: Event) {
        const enabled = (event.target as HTMLInputElement).checked;

        this.rules.update(rules =>
            rules.map(item => item.id === rule.id ? { ...item, enabled } : item),
        );
        this.changedRules.update(changes => {
            const next = new Map(changes);
            next.set(rule.id, enabled);
            return next;
        });
    }

    async createRule(event: Event) {
        event.preventDefault();

        const ticketKey = this.newTicketKey().trim();
        const requiredPlayers = this.newRequiredPlayers();

        if (!ticketKey || !Number.isInteger(requiredPlayers) || requiredPlayers < 2) {
            return;
        }

        this.status.set('Creating rule');

        try {
            await this.matchmaking.createAdminRule({
                ticket_key: ticketKey,
                required_players: requiredPlayers,
            });
            this.newTicketKey.set('');
            this.newRequiredPlayers.set(2);
            this.rules.set(await this.matchmaking.adminRules());
            await this.matchmaking.loadRules();
            this.status.set('Ready');
        } catch {
            this.status.set('Failed to create rule.');
        }
    }

    async saveRules() {
        const changes = Array.from(this.changedRules(), ([id, enabled]) => ({ id, enabled }));

        if (!changes.length) {
            return;
        }

        this.status.set('Saving rules');

        try {
            this.rules.set(await this.matchmaking.updateAdminRules(changes));
            await this.matchmaking.loadRules();
            this.changedRules.set(new Map());
            this.status.set('Ready');
        } catch {
            this.status.set('Failed to save rules.');
        }
    }

    profileName(firstName: string, lastName: string, fallbackId: string) {
        return [firstName, lastName].filter(Boolean).join(' ').trim() || this.shortId(fallbackId);
    }

    shortId(id: string) {
        return id.slice(0, 8);
    }
}
