import { Component, OnInit, inject, signal } from '@angular/core';
import { firstValueFrom } from 'rxjs';
import { BlockedUser, Friend, FriendRequestsResponse } from '../../models/social.model';
import { SocialService } from '../../services/social.service';

@Component({
    selector: 'app-friends',
    imports: [],
    templateUrl: './friends.component.html',
    styleUrl: './friends.component.scss',
})
export class FriendsComponent implements OnInit {
    private readonly social = inject(SocialService);

    readonly friends = signal<Friend[]>([]);
    readonly requests = signal<FriendRequestsResponse>({ incoming: [], outgoing: [] });
    readonly blockedUsers = signal<BlockedUser[]>([]);
    readonly status = signal('Ready');
    readonly busy = signal(false);

    ngOnInit() {
        void this.load();
    }

    async load() {
        if (this.busy()) {
            return;
        }

        this.busy.set(true);
        this.status.set('Loading');

        try {
            const [friends, requests, blocks] = await Promise.all([
                firstValueFrom(this.social.friends()),
                firstValueFrom(this.social.friendRequests()),
                firstValueFrom(this.social.blocks()),
            ]);
            this.friends.set(friends);
            this.requests.set(requests);
            this.blockedUsers.set(blocks);
            this.status.set('Ready');
        } catch {
            this.status.set('Failed to load friends.');
        } finally {
            this.busy.set(false);
        }
    }

    async accept(requestId: string) {
        this.status.set('Updating');
        await firstValueFrom(this.social.acceptFriendRequest(requestId));
        await this.load();
    }

    async decline(requestId: string) {
        this.status.set('Updating');
        await firstValueFrom(this.social.declineFriendRequest(requestId));
        await this.load();
    }

    async unblock(userId: string) {
        this.status.set('Updating');
        await firstValueFrom(this.social.unblockUser(userId));
        await this.load();
    }

    profileName(firstName: string, lastName: string, fallbackId: string) {
        return [firstName, lastName].filter(Boolean).join(' ').trim() || this.shortId(fallbackId);
    }

    shortId(id: string) {
        return id.slice(0, 8);
    }
}
