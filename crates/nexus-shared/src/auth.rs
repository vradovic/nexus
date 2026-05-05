use axum::http::{HeaderMap, header};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub email: String,
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

pub fn authenticated_user_id(headers: &HeaderMap, jwt_secret: &str) -> Result<Uuid, AppError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| AppError::unauthorized("missing authorization header"))?;
    let auth_header = auth_header
        .to_str()
        .map_err(|_| AppError::unauthorized("authorization header is invalid"))?;
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::unauthorized("authorization header must use Bearer token"))?;
    let claims = decode_access_token(token, jwt_secret)?;

    claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::unauthorized("invalid token subject"))
}
