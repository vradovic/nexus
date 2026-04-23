use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AuthAccount {
    pub id: i32,
    pub email: String,
    pub username: String,
}

#[derive(Debug, FromRow)]
pub struct AuthAccountWithPassword {
    pub id: i32,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: i32,
    pub email: String,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
}
