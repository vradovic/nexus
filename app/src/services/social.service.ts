import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { environment } from '../environments/environment';
import { Profile } from '../models/profile.model';
import {
    ActiveUserProfile,
    BlockedUser,
    ChatMessage,
    Friend,
    FriendRequestsResponse,
} from '../models/social.model';

@Injectable({ providedIn: 'root' })
export class SocialService {
    private readonly http = inject(HttpClient);

    me() {
        return this.http.get<Profile>(`${environment.socialApiUrl}/me`);
    }

    user(id: string) {
        return this.http.get<Profile>(`${environment.socialApiUrl}/users/${id}`);
    }

    friends() {
        return this.http.get<Friend[]>(`${environment.socialApiUrl}/friends`);
    }

    friendRequests() {
        return this.http.get<FriendRequestsResponse>(`${environment.socialApiUrl}/friend-requests`);
    }

    sendFriendRequest(recipientId: string) {
        return this.http.post(`${environment.socialApiUrl}/friend-requests`, {
            recipient_id: recipientId,
        });
    }

    acceptFriendRequest(requestId: string) {
        return this.http.post(`${environment.socialApiUrl}/friend-requests/${requestId}/accept`, {});
    }

    declineFriendRequest(requestId: string) {
        return this.http.post(`${environment.socialApiUrl}/friend-requests/${requestId}/decline`, {});
    }

    blocks() {
        return this.http.get<BlockedUser[]>(`${environment.socialApiUrl}/blocks`);
    }

    blockUser(blockedUserId: string) {
        return this.http.post<BlockedUser>(`${environment.socialApiUrl}/blocks`, {
            blocked_user_id: blockedUserId,
        });
    }

    unblockUser(blockedUserId: string) {
        return this.http.delete<void>(`${environment.socialApiUrl}/blocks/${blockedUserId}`);
    }

    chatMessages(channel: string, limit = 50) {
        const params = new URLSearchParams({ channel, limit: String(limit) });

        return this.http.get<ChatMessage[]>(`${environment.socialApiUrl}/chat/messages?${params}`);
    }

    sendChatMessage(channel: string, senderId: string, body: string) {
        return this.http.post<ChatMessage>(`${environment.socialApiUrl}/chat/messages`, {
            channel,
            sender: senderId,
            body,
        });
    }

    adminChatMessages() {
        return this.http.get<ChatMessage[]>(`${environment.socialApiUrl}/admin/chat/messages`);
    }

    activeUsers() {
        return this.http.get<ActiveUserProfile[]>(`${environment.realtimeApiUrl}/admin/active-users`);
    }
}
