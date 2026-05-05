use nexus_shared::{AppError, GameMode};
use redis::{AsyncCommands, Client};
use uuid::Uuid;

use crate::{
    models::MatchmakingTicket,
    redis_store,
};

#[derive(Clone)]
pub struct MatchmakingStore {
    redis_client: Client,
}

impl MatchmakingStore {
    pub fn new(redis_client: Client) -> Self {
        Self { redis_client }
    }

    pub async fn save_ticket(&self, ticket: &MatchmakingTicket) -> Result<(), AppError> {
        let ticket_key = ticket_key(ticket.id);
        redis_store::write_json(&self.redis_client, &ticket_key, ticket).await?;

        for user_id in &ticket.player_ids {
            self.save_user_ticket_mapping(*user_id, ticket.id).await?;
        }

        Ok(())
    }

    pub async fn find_ticket_by_id(
        &self,
        ticket_id: Uuid,
    ) -> Result<Option<MatchmakingTicket>, AppError> {
        redis_store::read_json(&self.redis_client, &ticket_key(ticket_id)).await
    }

    pub async fn find_ticket_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<MatchmakingTicket>, AppError> {
        let ticket_id = self.find_ticket_id_by_user_id(user_id).await?;
        match ticket_id {
            Some(ticket_id) => self.find_ticket_by_id(ticket_id).await,
            None => Ok(None),
        }
    }

    pub async fn enqueue_ticket(&self, game_mode: &GameMode, ticket_id: Uuid) -> Result<(), AppError> {
        let queue_key = queue_key(game_mode);
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

    pub async fn peek_queue(
        &self,
        game_mode: &GameMode,
        limit: isize,
    ) -> Result<Vec<MatchmakingTicket>, AppError> {
        let queue_key = queue_key(game_mode);
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
        game_mode: &GameMode,
        count: usize,
    ) -> Result<(), AppError> {
        if count == 0 {
            return Ok(());
        }

        let queue_key = queue_key(game_mode);
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

    async fn save_user_ticket_mapping(&self, user_id: Uuid, ticket_id: Uuid) -> Result<(), AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        connection
            .set::<String, String, ()>(user_ticket_key(user_id), ticket_id.to_string())
            .await
            .map_err(|_| AppError::internal("failed to save user matchmaking mapping"))?;

        Ok(())
    }

    async fn find_ticket_id_by_user_id(&self, user_id: Uuid) -> Result<Option<Uuid>, AppError> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| AppError::internal("failed to connect to redis"))?;

        let ticket_id = connection
            .get::<String, Option<String>>(user_ticket_key(user_id))
            .await
            .map_err(|_| AppError::internal("failed to read user matchmaking mapping"))?;

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

fn user_ticket_key(user_id: Uuid) -> String {
    format!("matchmaking:user:{user_id}")
}

fn queue_key(game_mode: &GameMode) -> String {
    format!("matchmaking:queues:{}", game_mode.redis_queue_key())
}
