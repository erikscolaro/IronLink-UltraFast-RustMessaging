use crate::core::{AppError, AppState};
use axum::extract::State;
use axum::{
    Error,
    body::Body,
    extract::Request,
    http,
    http::{Response, StatusCode},
    middleware::Next,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// struct che codifica il contenuto del token jwt
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize, // Expiry time of the token
    pub iat: usize, // Issued at time of the token
    pub id: i32,
    pub username: String,
}

pub fn encode_jwt(username: String, id: i32, secret: &String) -> Result<String, Error> {
    let now = Utc::now();
    let expire: chrono::TimeDelta = Duration::hours(24);
    let exp: usize = (now + expire).timestamp() as usize;
    let iat: usize = now.timestamp() as usize;
    let claim = Claims {
        iat,
        exp,
        username,
        id,
    };

    encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|_| Error::new("Error in encoding jwt token"))
}

pub fn decode_jwt(jwt_token: String, secret: &String) -> Result<TokenData<Claims>, Error> {
    decode(
        &jwt_token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| Error::new("Error in decoding jwt token"))
}

pub async fn authentication_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response<Body>, AppError> {
    let auth_header = req.headers_mut().get(http::header::AUTHORIZATION);
    let auth_header = match auth_header {
        Some(header) => header.to_str().map_err(|_| AppError {
            code: StatusCode::FORBIDDEN,
            message: Option::from("Empty header is not allowed".to_string()),
            details: None,
        })?,
        None => {
            return Err(AppError {
                code: StatusCode::FORBIDDEN,
                message: Option::from("Please add the JWT token to the header".to_string()),
                details: None,
            });
        }
    };
    let mut header = auth_header.split_whitespace();
    let (bearer, token) = (header.next(), header.next());
    let token_data = match decode_jwt(token.unwrap().to_string(), &state.jwt_secret) {
        Ok(data) => data,
        Err(_) => {
            return Err(AppError {
                code: StatusCode::UNAUTHORIZED,
                message: Option::from("Unable to decode token".to_string()),
                details: None,
            });
        }
    };

    // Fetch the user details from the database
    let current_user = match state
        .user
        .find_by_username(&token_data.claims.username)
        .await?
    {
        Some(user) => user,
        None => {
            return Err(AppError {
                code: StatusCode::UNAUTHORIZED,
                message: Option::from("You are not an authorized user".to_string()),
                details: None,
            });
        }
    };
    req.extensions_mut().insert(current_user);
    // voledo si può recuperare lo user da extension
    Ok(next.run(req).await)
}
