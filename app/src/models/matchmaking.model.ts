export interface MatchmakingTicket {
    id: string;
    player_id: string;
    ticket_key: string;
}

export interface PendingMatch {
    id: string;
    rule_id: string;
    ticket_key: string;
    player_ids: string[];
    confirmed_player_ids: string[];
    expires_at_unix_seconds: number;
}

export interface MatchmakingRule {
    id: string;
    ticket_key: string;
    required_players: number;
    enabled: boolean;
}

export interface MatchmakingStatus {
    ticket: MatchmakingTicket | null;
    pending_match: PendingMatch | null;
}

export interface CreateMatchmakingRuleRequest {
    ticket_key: string;
    required_players: number;
}

export interface UpdateMatchmakingRuleEnabled {
    id: string;
    enabled: boolean;
}
