use sqlx::PgPool;

use crate::error::AppError;
use crate::models::AuthAccount;

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
        email: &str,
        username: &str,
        password_hash: &str,
    ) -> Result<AuthAccount, AppError> {
        sqlx::query_as::<_, AuthAccount>(
            r#"
            insert into auth_accounts (email, username, password_hash)
            values ($1, $2, $3)
            returning id, email, username
            "#,
        )
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
}
