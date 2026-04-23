use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{UserProfile, UserRegisteredEvent};

#[derive(Clone)]
pub struct UserProfileRepository {
    db: PgPool,
}

impl UserProfileRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn upsert_user_profile(
        &self,
        event: &UserRegisteredEvent,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            insert into user_profiles (id, first_name, last_name)
            values ($1, $2, $3)
            on conflict (id) do update
            set first_name = excluded.first_name,
                last_name = excluded.last_name
            "#,
        )
        .bind(event.user_id)
        .bind(&event.first_name)
        .bind(&event.last_name)
        .execute(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        Ok(())
    }

    pub async fn find_user_profile_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<UserProfile>, AppError> {
        sqlx::query_as::<_, UserProfile>(
            r#"
            select id, first_name, last_name
            from user_profiles
            where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }
}
