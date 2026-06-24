use nexus_shared::AppError;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{FriendRequest, UserProfile, UserRegisteredEvent};

#[derive(Clone)]
pub struct UserProfileRepository {
    db: PgPool,
}

impl UserProfileRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn upsert_user_profile(&self, event: &UserRegisteredEvent) -> Result<(), AppError> {
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

    pub async fn find_user_profile_by_id(&self, id: Uuid) -> Result<Option<UserProfile>, AppError> {
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

    pub async fn create_friend_request(
        &self,
        requester_id: Uuid,
        recipient_id: Uuid,
    ) -> Result<FriendRequest, AppError> {
        sqlx::query_as::<_, FriendRequest>(
            r#"
            insert into friend_requests (id, requester_id, recipient_id, status)
            values ($1, $2, $3, 'pending')
            returning id, requester_id, recipient_id, status
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(requester_id)
        .bind(recipient_id)
        .fetch_one(&self.db)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(db_error)
                if db_error.is_unique_violation()
                    || db_error.constraint() == Some("friend_requests_check") =>
            {
                AppError::conflict("friend request already exists")
            }
            sqlx::Error::Database(db_error) if db_error.is_foreign_key_violation() => {
                AppError::not_found("user profile not found")
            }
            _ => AppError::internal("database operation failed"),
        })
    }

    pub async fn decline_friend_request(
        &self,
        request_id: Uuid,
        recipient_id: Uuid,
    ) -> Result<Option<FriendRequest>, AppError> {
        sqlx::query_as::<_, FriendRequest>(
            r#"
            update friend_requests
            set status = 'declined',
                responded_at = now()
            where id = $1
              and recipient_id = $2
              and status = 'pending'
            returning id, requester_id, recipient_id, status
            "#,
        )
        .bind(request_id)
        .bind(recipient_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }
}
