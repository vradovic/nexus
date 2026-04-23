use async_nats::{Client, jetstream};
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng},
};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::error::AppError;
use crate::messaging::USER_REGISTERED_SUBJECT;
use crate::models::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse};
use crate::repositories::auth_repository::AuthRepository;

pub async fn register_user(
    auth_repository: &AuthRepository,
    nats_client: &Client,
    payload: RegisterRequest,
) -> Result<RegisterResponse, AppError> {
    validate_registration(&payload)?;

    let password_hash = hash_password(&payload.password)?;
    let user_id = Uuid::new_v4();

    let auth_account = auth_repository
        .create_auth_account(user_id, &payload.email, &payload.username, &password_hash)
        .await?;

    publish_user_registered_event(nats_client, &auth_account, &payload).await?;

    Ok(RegisterResponse {
        id: auth_account.id,
        email: auth_account.email,
        username: auth_account.username,
    })
}

pub async fn login_user(
    auth_repository: &AuthRepository,
    jwt_secret: &str,
    payload: LoginRequest,
) -> Result<LoginResponse, AppError> {
    validate_login(&payload)?;

    let auth_account = auth_repository
        .find_auth_account_by_email(&payload.email)
        .await?
        .ok_or_else(|| AppError::unauthorized("invalid email or password"))?;

    verify_password(&payload.password, &auth_account.password_hash)?;

    let access_token = create_access_token(auth_account.id, &auth_account.email, jwt_secret)?;

    Ok(LoginResponse { access_token })
}

fn validate_registration(payload: &RegisterRequest) -> Result<(), AppError> {
    if !payload.email.contains('@') {
        return Err(AppError::bad_request("email must contain '@'"));
    }

    if payload.username.trim().len() < 3 {
        return Err(AppError::bad_request(
            "username must be at least 3 characters long",
        ));
    }

    if payload.password.len() < 8 {
        return Err(AppError::bad_request(
            "password must be at least 8 characters long",
        ));
    }

    if payload.first_name.trim().is_empty() {
        return Err(AppError::bad_request("first_name is required"));
    }

    if payload.last_name.trim().is_empty() {
        return Err(AppError::bad_request("last_name is required"));
    }

    Ok(())
}

fn validate_login(payload: &LoginRequest) -> Result<(), AppError> {
    if payload.email.trim().is_empty() {
        return Err(AppError::bad_request("email is required"));
    }

    if payload.password.is_empty() {
        return Err(AppError::bad_request("password is required"));
    }

    Ok(())
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::internal("failed to hash password"))
}

fn verify_password(password: &str, stored_hash: &str) -> Result<(), AppError> {
    let parsed_hash =
        PasswordHash::new(stored_hash).map_err(|_| AppError::internal("stored password hash is invalid"))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::unauthorized("invalid email or password"))
}

async fn publish_user_registered_event(
    nats_client: &Client,
    auth_account: &crate::models::AuthAccount,
    payload: &RegisterRequest,
) -> Result<(), AppError> {
    let event = UserRegisteredEvent {
        user_id: auth_account.id,
        first_name: payload.first_name.clone(),
        last_name: payload.last_name.clone(),
    };

    let body = serde_json::to_vec(&event)
        .map_err(|_| AppError::internal("failed to serialize registration event"))?;

    jetstream::new(nats_client.clone())
        .publish(USER_REGISTERED_SUBJECT, body.into())
        .await
        .map_err(|_| AppError::internal("failed to publish registration event"))?;

    Ok(())
}

fn create_access_token(user_id: Uuid, email: &str, jwt_secret: &str) -> Result<String, AppError> {
    let expiration = SystemTime::now()
        .checked_add(Duration::from_secs(60 * 60))
        .ok_or_else(|| AppError::internal("failed to calculate token expiration"))?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::internal("system clock is invalid"))?
        .as_secs() as usize;

    let claims = AccessTokenClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|_| AppError::internal("failed to generate access token"))
}

#[derive(Serialize)]
struct AccessTokenClaims {
    sub: String,
    email: String,
    exp: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserRegisteredEvent {
    user_id: Uuid,
    first_name: String,
    last_name: String,
}

#[cfg(test)]
mod tests {
    use super::{validate_login, validate_registration};
    use crate::models::{LoginRequest, RegisterRequest};

    fn request(
        email: &str,
        username: &str,
        first_name: &str,
        last_name: &str,
        password: &str,
    ) -> RegisterRequest {
        RegisterRequest {
            email: email.to_string(),
            username: username.to_string(),
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            password: password.to_string(),
        }
    }

    fn login_request(email: &str, password: &str) -> LoginRequest {
        LoginRequest {
            email: email.to_string(),
            password: password.to_string(),
        }
    }

    #[test]
    fn accepts_valid_registration_payload() {
        let payload = request("player@example.com", "player1", "John", "Doe", "supersecret");

        let result = validate_registration(&payload);

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_short_password() {
        let payload = request("player@example.com", "player1", "John", "Doe", "short");

        let result = validate_registration(&payload);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_first_name() {
        let payload = request("player@example.com", "player1", "", "Doe", "supersecret");

        let result = validate_registration(&payload);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_login_without_email() {
        let payload = login_request("", "supersecret");

        let result = validate_login(&payload);

        assert!(result.is_err());
    }
}
