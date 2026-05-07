use axum::http::{HeaderMap, header};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

use crate::AppError;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Player,
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Admin => "admin",
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for UserRole {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "player" => Ok(Self::Player),
            "admin" => Ok(Self::Admin),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub email: String,
    pub role: UserRole,
    pub exp: usize,
}

pub fn decode_access_token(
    token: &str,
    jwt_secret: &str,
) -> Result<AccessTokenClaims, AppError> {
    decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map(|token_data| token_data.claims)
    .map_err(|_| AppError::unauthorized("invalid access token"))
}

pub fn authenticated_user(
    headers: &HeaderMap,
    jwt_secret: &str,
) -> Result<AuthenticatedUser, AppError> {
    let token = bearer_token_from_headers(headers)?;
    authenticated_user_from_token(token, jwt_secret)
}

pub fn authenticated_user_id(headers: &HeaderMap, jwt_secret: &str) -> Result<Uuid, AppError> {
    authenticated_user(headers, jwt_secret).map(|user| user.user_id)
}

fn bearer_token_from_headers(headers: &HeaderMap) -> Result<&str, AppError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| AppError::unauthorized("missing authorization header"))?;
    let auth_header = auth_header
        .to_str()
        .map_err(|_| AppError::unauthorized("authorization header is invalid"))?;
    auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::unauthorized("authorization header must use Bearer token"))
}

pub fn authenticated_user_from_token(
    token: &str,
    jwt_secret: &str,
) -> Result<AuthenticatedUser, AppError> {
    let claims = decode_access_token(token, jwt_secret)?;
    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::unauthorized("invalid token subject"))?;

    Ok(AuthenticatedUser {
        user_id,
        email: claims.email,
        role: claims.role,
    })
}
