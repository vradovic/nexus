use sqlx::PgPool;

use crate::model::users::User;

pub async fn get_all_users(pool: &PgPool) -> Vec<User> {
    sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(pool)
        .await
        .unwrap_or_else(|_| vec![])
}
