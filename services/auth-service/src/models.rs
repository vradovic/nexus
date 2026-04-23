use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AuthAccount {
    pub id: Uuid,
    pub email: String,
    pub username: String,
}

#[derive(Debug, FromRow)]
pub struct AuthAccountWithPassword {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
}
