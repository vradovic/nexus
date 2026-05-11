use nexus_shared::AppError;
use sqlx::PgPool;
use uuid::Uuid;

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

    pub async fn create_rule(
        &self,
        ticket_key: &str,
        required_players: i32,
    ) -> Result<MatchmakingRule, AppError> {
        sqlx::query_as::<_, MatchmakingRule>(
            r#"
            insert into matchmaking_rules (id, ticket_key, required_players, enabled)
            values ($1, $2, $3, true)
            returning id, ticket_key, required_players
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ticket_key)
        .bind(required_players)
        .fetch_one(&self.db)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(db_error) if db_error.is_unique_violation() => {
                AppError::conflict("ticket_key already exists")
            }
            _ => AppError::internal("database operation failed"),
        })
    }
}
