use async_nats::Client as NatsClient;
use redis::Client as RedisClient;
use sqlx::PgPool;

use crate::{repository::MatchmakingRuleRepository, store::MatchmakingStore};

#[derive(Clone)]
pub struct AppState {
    pub matchmaking_store: MatchmakingStore,
    pub matchmaking_rule_repository: MatchmakingRuleRepository,
    pub jwt_secret: String,
    pub nats_client: NatsClient,
}

impl AppState {
    pub fn new(
        db: PgPool,
        redis_client: RedisClient,
        jwt_secret: String,
        nats_client: NatsClient,
    ) -> Self {
        Self {
            matchmaking_store: MatchmakingStore::new(redis_client),
            matchmaking_rule_repository: MatchmakingRuleRepository::new(db),
            jwt_secret,
            nats_client,
        }
    }
}
