export interface Friend {
    friendship_id: string;
    friend_id: string;
    first_name: string;
    last_name: string;
}

export interface FriendRequest {
    id: string;
    requester_id: string;
    requester_first_name: string;
    requester_last_name: string;
    recipient_id: string;
    recipient_first_name: string;
    recipient_last_name: string;
    status: string;
}

export interface FriendRequestsResponse {
    incoming: FriendRequest[];
    outgoing: FriendRequest[];
}

export interface BlockedUser {
    block_id: string;
    blocked_user_id: string;
    first_name: string;
    last_name: string;
}

export interface ChatMessage {
    id: string;
    channel: string;
    sender_id: string;
    body: string;
    created_at: string;
}

export interface ActiveUserProfile {
    id: string;
    first_name: string;
    last_name: string;
}
