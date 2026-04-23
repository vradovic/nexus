use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{AuthAccount, AuthAccountWithPassword};

#[derive(Clone)]
pub struct AuthRepository {
    db: PgPool,
}

impl AuthRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_auth_account(
        &self,
        id: Uuid,
        email: &str,
        username: &str,
        password_hash: &str,
    ) -> Result<AuthAccount, AppError> {
        sqlx::query_as::<_, AuthAccount>(
            r#"
            insert into auth_accounts (id, email, username, password_hash)
            values ($1, $2, $3, $4)
            returning id, email, username
            "#,
        )
        .bind(id)
        .bind(email)
        .bind(username)
        .bind(password_hash)
        .fetch_one(&self.db)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(db_error) if db_error.is_unique_violation() => {
                AppError::conflict("email or username already exists")
            }
            _ => AppError::internal("database operation failed"),
        })
    }

    pub async fn find_auth_account_by_email(
        &self,
        email: &str,
    ) -> Result<Option<AuthAccountWithPassword>, AppError> {
        sqlx::query_as::<_, AuthAccountWithPassword>(
            r#"
            select id, email, password_hash
            from auth_accounts
            where email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }
}
