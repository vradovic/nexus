use nexus_shared::{AppError, read_json, write_json};
use redis::{AsyncCommands, Client};
use uuid::Uuid;
use crate::models::MatchmakingTicket;

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

    pub async fn enqueue_ticket(&self, ticket_key_value: &str, ticket_id: Uuid) -> Result<(), AppError> {
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

    async fn save_player_ticket_mapping(&self, player_id: Uuid, ticket_id: Uuid) -> Result<(), AppError> {
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
