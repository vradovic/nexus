use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};

use crate::error::AppError;
use crate::models::{RegisterRequest, RegisterResponse};
use crate::repositories::auth_repository::AuthRepository;

pub async fn register_user(
    auth_repository: &AuthRepository,
    payload: RegisterRequest,
) -> Result<RegisterResponse, AppError> {
    validate_registration(&payload)?;

    let password_hash = hash_password(&payload.password)?;

    let auth_account = auth_repository
        .create_auth_account(&payload.email, &payload.username, &password_hash)
        .await?;

    Ok(RegisterResponse {
        id: auth_account.id,
        email: auth_account.email,
        username: auth_account.username,
    })
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

    Ok(())
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::internal("failed to hash password"))
}

#[cfg(test)]
mod tests {
    use super::validate_registration;
    use crate::models::RegisterRequest;

    fn request(email: &str, username: &str, password: &str) -> RegisterRequest {
        RegisterRequest {
            email: email.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    #[test]
    fn accepts_valid_registration_payload() {
        let payload = request("player@example.com", "player1", "supersecret");

        let result = validate_registration(&payload);

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_short_password() {
        let payload = request("player@example.com", "player1", "short");

        let result = validate_registration(&payload);

        assert!(result.is_err());
    }
}
