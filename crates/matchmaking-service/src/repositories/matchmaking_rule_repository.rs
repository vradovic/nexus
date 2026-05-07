use nexus_shared::AppError;
use sqlx::PgPool;

use crate::models::MatchmakingRule;

#[derive(Clone)]
pub struct MatchmakingRuleRepository {
    db: PgPool,
}

impl MatchmakingRuleRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn find_enabled_rules(&self) -> Result<Vec<MatchmakingRule>, AppError> {
        sqlx::query_as::<_, MatchmakingRule>(
            r#"
            select id, ticket_key, required_players
            from matchmaking_rules
            where enabled = true
            order by ticket_key
            "#,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }

    pub async fn find_enabled_rule_by_ticket_key(
        &self,
        ticket_key: &str,
    ) -> Result<Option<MatchmakingRule>, AppError> {
        sqlx::query_as::<_, MatchmakingRule>(
            r#"
            select id, ticket_key, required_players
            from matchmaking_rules
            where ticket_key = $1 and enabled = true
            "#,
        )
        .bind(ticket_key)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }
}
