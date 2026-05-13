use nexus_shared::{AppError, read_json, write_json};
use redis::{AsyncCommands, Client};
use uuid::Uuid;

use crate::models::{MatchmakingTicket, PendingMatch};

const PENDING_MATCH_TTL_GRACE_SECONDS: u64 = 5;

#[derive(Clone)]
pub struct MatchmakingStore {
    redis_client: Client,
}

impl MatchmakingStore {
    pub fn new(redis_client: Client) -> Self {
        Self { redis_client }
    }

    pub async fn save_ticket(&self, ticket: &MatchmakingTicket) -> Result<(), AppError> {
        write_json(&self.redis_client, &ticket_key(ticket.id), ticket).await?;
        self.save_player_ticket_mapping(ticket.player_id, ticket.id).await
    }

    pub async fn find_ticket_by_id(
        &self,
        ticket_id: Uuid,
    ) -> Result<Option<MatchmakingTicket>, AppError> {
        read_json(&self.redis_client, &ticket_key(ticket_id)).await
    }

    pub async fn find_ticket_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Option<MatchmakingTicket>, AppError> {
        let ticket_id = self.find_ticket_id_by_player_id(player_id).await?;

        match ticket_id {
            Some(ticket_id) => self.find_ticket_by_id(ticket_id).await,
            None => Ok(None),
        }
    }

    pub async fn enqueue_ticket(
        &self,
        ticket_key_value: &str,
        ticket_id: Uuid,
    ) -> Result<(), AppError> {
        let queue_key = queue_key(ticket_key_value);
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .rpush::<String, String, ()>(queue_key, ticket_id.to_string())
            .await
            .map_err(|_| AppError::internal("failed to enqueue matchmaking ticket"))?;

        Ok(())
    }

    pub async fn remove_ticket_from_queue(
        &self,
        ticket_key_value: &str,
        ticket_id: Uuid,
    ) -> Result<(), AppError> {
        let queue_key = queue_key(ticket_key_value);
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .lrem::<String, String, ()>(queue_key, 1, ticket_id.to_string())
            .await
            .map_err(|_| AppError::internal("failed to remove matchmaking ticket from queue"))?;

        Ok(())
    }

    pub async fn peek_queue(
        &self,
        ticket_key_value: &str,
        limit: isize,
    ) -> Result<Vec<MatchmakingTicket>, AppError> {
        let queue_key = queue_key(ticket_key_value);
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        let ticket_ids = connection
            .lrange::<String, Vec<String>>(queue_key, 0, limit - 1)
            .await
            .map_err(|_| AppError::internal("failed to read matchmaking queue"))?;

        let mut tickets = Vec::new();
        for ticket_id in ticket_ids {
            let ticket_id = Uuid::parse_str(&ticket_id)
                .map_err(|_| AppError::internal("stored matchmaking ticket id is invalid"))?;

            if let Some(ticket) = self.find_ticket_by_id(ticket_id).await? {
                tickets.push(ticket);
            }
        }

        Ok(tickets)
    }

    pub async fn remove_queue_prefix(
        &self,
        ticket_key_value: &str,
        count: usize,
    ) -> Result<(), AppError> {
        if count == 0 {
            return Ok(());
        }

        let queue_key = queue_key(ticket_key_value);
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .ltrim::<String, ()>(queue_key, count as isize, -1)
            .await
            .map_err(|_| AppError::internal("failed to trim matchmaking queue"))?;

        Ok(())
    }

    pub async fn delete_ticket(&self, ticket: &MatchmakingTicket) -> Result<(), AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .del::<String, ()>(ticket_key(ticket.id))
            .await
            .map_err(|_| AppError::internal("failed to delete matchmaking ticket"))?;

        connection
            .del::<String, ()>(player_ticket_key(ticket.player_id))
            .await
            .map_err(|_| AppError::internal("failed to delete player matchmaking mapping"))?;

        Ok(())
    }

    pub async fn save_pending_match(&self, pending_match: &PendingMatch) -> Result<(), AppError> {
        let match_key = pending_match_key(pending_match.id);
        let ttl_seconds = pending_match_ttl_seconds(pending_match);

        write_json(&self.redis_client, &match_key, pending_match).await?;

        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .expire::<String, ()>(match_key.clone(), ttl_seconds)
            .await
            .map_err(|_| AppError::internal("failed to set pending match ttl"))?;

        connection
            .sadd::<String, String, ()>(active_pending_matches_key(), pending_match.id.to_string())
            .await
            .map_err(|_| AppError::internal("failed to index pending match"))?;

        for player_id in &pending_match.player_ids {
            let player_match_key = player_pending_match_key(*player_id);

            connection
                .set::<String, String, ()>(player_match_key.clone(), pending_match.id.to_string())
                .await
                .map_err(|_| AppError::internal("failed to save player pending match mapping"))?;

            connection
                .expire::<String, ()>(player_match_key, ttl_seconds)
                .await
                .map_err(|_| AppError::internal("failed to set player pending match ttl"))?;
        }

        Ok(())
    }

    pub async fn find_pending_match_by_id(
        &self,
        match_id: Uuid,
    ) -> Result<Option<PendingMatch>, AppError> {
        read_json(&self.redis_client, &pending_match_key(match_id)).await
    }

    pub async fn find_pending_match_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Option<PendingMatch>, AppError> {
        let pending_match_id = self.find_pending_match_id_by_player_id(player_id).await?;

        let Some(pending_match_id) = pending_match_id else {
            return Ok(None);
        };

        let pending_match = self.find_pending_match_by_id(pending_match_id).await?;
        if pending_match.is_none() {
            self.delete_player_pending_match_mapping(player_id).await?;
        }

        Ok(pending_match)
    }

    pub async fn find_all_pending_matches(&self) -> Result<Vec<PendingMatch>, AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        let pending_match_ids = connection
            .smembers::<String, Vec<String>>(active_pending_matches_key())
            .await
            .map_err(|_| AppError::internal("failed to read pending matches index"))?;

        let mut pending_matches = Vec::new();
        for pending_match_id in pending_match_ids {
            let pending_match_id = Uuid::parse_str(&pending_match_id)
                .map_err(|_| AppError::internal("stored pending match id is invalid"))?;

            match self.find_pending_match_by_id(pending_match_id).await? {
                Some(pending_match) => pending_matches.push(pending_match),
                None => self
                    .remove_pending_match_from_active_set(pending_match_id)
                    .await?,
            }
        }

        Ok(pending_matches)
    }

    pub async fn delete_pending_match(&self, pending_match: &PendingMatch) -> Result<(), AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .del::<String, ()>(pending_match_key(pending_match.id))
            .await
            .map_err(|_| AppError::internal("failed to delete pending match"))?;

        connection
            .srem::<String, String, ()>(
                active_pending_matches_key(),
                pending_match.id.to_string(),
            )
            .await
            .map_err(|_| AppError::internal("failed to remove pending match from index"))?;

        for player_id in &pending_match.player_ids {
            connection
                .del::<String, ()>(player_pending_match_key(*player_id))
                .await
                .map_err(|_| AppError::internal("failed to delete player pending match mapping"))?;
        }

        Ok(())
    }

    async fn save_player_ticket_mapping(
        &self,
        player_id: Uuid,
        ticket_id: Uuid,
    ) -> Result<(), AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .set::<String, String, ()>(player_ticket_key(player_id), ticket_id.to_string())
            .await
            .map_err(|_| AppError::internal("failed to save player matchmaking mapping"))?;

        Ok(())
    }

    async fn find_ticket_id_by_player_id(&self, player_id: Uuid) -> Result<Option<Uuid>, AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        let ticket_id = connection
            .get::<String, Option<String>>(player_ticket_key(player_id))
            .await
            .map_err(|_| AppError::internal("failed to read player matchmaking mapping"))?;

        ticket_id
            .map(|ticket_id| {
                Uuid::parse_str(&ticket_id)
                    .map_err(|_| AppError::internal("stored matchmaking ticket id is invalid"))
            })
            .transpose()
    }

    async fn find_pending_match_id_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Option<Uuid>, AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        let pending_match_id = connection
            .get::<String, Option<String>>(player_pending_match_key(player_id))
            .await
            .map_err(|_| AppError::internal("failed to read player pending match mapping"))?;

        pending_match_id
            .map(|pending_match_id| {
                Uuid::parse_str(&pending_match_id)
                    .map_err(|_| AppError::internal("stored pending match id is invalid"))
            })
            .transpose()
    }

    async fn delete_player_pending_match_mapping(&self, player_id: Uuid) -> Result<(), AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .del::<String, ()>(player_pending_match_key(player_id))
            .await
            .map_err(|_| AppError::internal("failed to delete player pending match mapping"))?;

        Ok(())
    }

    async fn remove_pending_match_from_active_set(&self, match_id: Uuid) -> Result<(), AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .srem::<String, String, ()>(active_pending_matches_key(), match_id.to_string())
            .await
            .map_err(|_| AppError::internal("failed to remove stale pending match from index"))?;

        Ok(())
    }
}

fn ticket_key(ticket_id: Uuid) -> String {
    format!("matchmaking:tickets:{ticket_id}")
}

fn player_ticket_key(player_id: Uuid) -> String {
    format!("matchmaking:player:{player_id}")
}

fn queue_key(ticket_key_value: &str) -> String {
    format!("matchmaking:queues:{ticket_key_value}")
}

fn pending_match_key(match_id: Uuid) -> String {
    format!("matchmaking:pending_matches:{match_id}")
}

fn player_pending_match_key(player_id: Uuid) -> String {
    format!("matchmaking:pending_match_player:{player_id}")
}

fn active_pending_matches_key() -> String {
    "matchmaking:pending_matches:active".to_string()
}

fn pending_match_ttl_seconds(pending_match: &PendingMatch) -> i64 {
    let now = current_unix_timestamp();
    let ttl = pending_match
        .expires_at_unix_seconds
        .saturating_sub(now)
        .saturating_add(PENDING_MATCH_TTL_GRACE_SECONDS);

    if ttl == 0 { 1 } else { ttl as i64 }
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
