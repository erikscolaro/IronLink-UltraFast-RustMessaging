use crate::core::{AppError, AppState};
use crate::entities::{User, UserChatMetadata, UserRole};
use crate::repositories::Read;
use axum::extract::State;
use axum::{Error, body::Body, extract::Request, http, http::Response, middleware::Next};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};

// struct che codifica il contenuto del token jwt
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize, // Expiry time of the token
    pub iat: usize, // Issued at time of the token
    pub id: i32,
    pub username: String,
}

#[instrument(skip(secret), fields(username = %username, id = %id))]
pub fn encode_jwt(username: String, id: i32, secret: &String) -> Result<String, Error> {
    debug!("Encoding JWT token for user");
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
    .map(|token| {
        info!("JWT token encoded successfully");
        token
    })
    .map_err(|e| {
        error!("Failed to encode JWT token: {:?}", e);
        Error::new("Error in encoding jwt token")
    })
}

#[instrument(skip(jwt_token, secret))]
pub fn decode_jwt(jwt_token: String, secret: &String) -> Result<TokenData<Claims>, Error> {
    debug!("Decoding JWT token");
    decode(
        &jwt_token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map(|data| {
        info!("JWT token decoded successfully for user: {}", data.claims.username);
        data
    })
    .map_err(|e| {
        error!("Failed to decode JWT token: {:?}", e);
        Error::new("Error in decoding jwt token")
    })
}

#[instrument(skip(state, req, next))]
pub async fn authentication_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response<Body>, AppError> {
    debug!("Running authentication middleware");
    let auth_header = req.headers_mut().get(http::header::AUTHORIZATION);
    let auth_header = match auth_header {
        Some(header) => header
            .to_str()
            .map_err(|_| {
                warn!("Invalid authorization header format");
                AppError::forbidden("Empty header is not allowed")
            })?,
        None => {
            warn!("Missing authorization header");
            return Err(AppError::forbidden(
                "Please add the JWT token to the header",
            ));
        }
    };
    let mut header = auth_header.split_whitespace();
    let (bearer, token) = (header.next(), header.next());
    let token_data = match decode_jwt(token.unwrap().to_string(), &state.jwt_secret) {
        Ok(data) => data,
        Err(_) => {
            warn!("Failed to decode JWT token");
            return Err(AppError::unauthorized("Unable to decode token"));
        }
    };

    // Fetch the user details from the database
    let current_user = match state
        .user
        .find_by_username(&token_data.claims.username)
        .await?
    {
        Some(user) => {
            info!("User authenticated: {}", user.username);
            user
        }
        None => {
            warn!("User not found in database: {}", token_data.claims.username);
            return Err(AppError::unauthorized("You are not an authorized user"));
        }
    };
    req.extensions_mut().insert(current_user);
    // voledo si può recuperare lo user da extension
    Ok(next.run(req).await)
}

/// Middleware che verifica che l'utente corrente sia membro della chat specificata
/// Estrae chat_id dal path, verifica la membership tramite metadata e inserisce il metadata nell'Extension
#[instrument(skip(state, req, next))]
pub async fn chat_membership_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response<Body>, AppError> {
    debug!("Running chat membership middleware");
    // 1. Ottenere l'utente corrente dall'Extension (deve essere stato inserito dall'authentication_middleware)
    let current_user = req
        .extensions()
        .get::<User>()
        .ok_or_else(|| {
            warn!("User not found in request extensions");
            AppError::unauthorized("User not authenticated")
        })?
        .clone();

    // 2. Estrarre chat_id dal path
    let chat_id: i32 = req
        .uri()
        .path()
        .split('/')
        .find_map(|segment| segment.parse::<i32>().ok())
        .ok_or_else(|| {
            warn!("Chat ID not found in path: {}", req.uri().path());
            AppError::bad_request("Chat ID not found in path")
        })?;

    debug!("Checking membership for user {} in chat {}", current_user.user_id, chat_id);
    
    // 3. Verificare che l'utente sia membro della chat tramite metadata
    let metadata = state
        .meta
        .read(&(current_user.user_id, chat_id))
        .await?
        .ok_or_else(|| {
            warn!("User {} is not a member of chat {}", current_user.user_id, chat_id);
            AppError::forbidden("You are not a member of this chat")
        })?;

    info!("User {} verified as member of chat {}", current_user.user_id, chat_id);
    
    // 4. Inserire il metadata nell'Extension per uso successivo negli handler
    req.extensions_mut().insert(metadata);

    Ok(next.run(req).await)
}

/// Helper function per verificare che un utente abbia uno dei ruoli richiesti
///
/// # Arguments
/// * `metadata` - Il metadata dell'utente da verificare
/// * `allowed_roles` - Lista di ruoli permessi
///
/// # Returns
/// * `Ok(())` se il ruolo è permesso
/// * `Err(AppError)` se il ruolo non è tra quelli permessi
#[instrument(skip(metadata))]
pub fn require_role(
    metadata: &UserChatMetadata,
    allowed_roles: &[UserRole],
) -> Result<(), AppError> {
    debug!("Checking role requirements for user {} in chat {}", metadata.user_id, metadata.chat_id);
    let user_role = metadata
        .user_role
        .as_ref()
        .ok_or_else(|| {
            warn!("User role not found in metadata for user {}", metadata.user_id);
            AppError::forbidden("User role not found in metadata")
        })?;

    if !allowed_roles.contains(user_role) {
        warn!(
            "User {} has insufficient role {:?}, required one of: {:?}",
            metadata.user_id, user_role, allowed_roles
        );
        return Err(AppError::forbidden("Insufficient role").with_details(format!(
            "This action requires one of the following roles: {:?}",
            allowed_roles
        )));
    }

    info!("Role check passed for user {} with role {:?}", metadata.user_id, user_role);
    Ok(())
}
