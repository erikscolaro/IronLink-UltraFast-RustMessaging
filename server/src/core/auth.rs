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
pub fn encode_jwt(username: &String, id: i32, secret: &String) -> Result<String, Error> {
    debug!("Encoding JWT token for user");
    let now = Utc::now();
    let expire: chrono::TimeDelta = Duration::hours(24);
    let exp: usize = (now + expire).timestamp() as usize;
    let iat: usize = now.timestamp() as usize;
    let claim = Claims {
        iat,
        exp,
        username: username.clone(),
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
pub fn decode_jwt(jwt_token: &String, secret: &String) -> Result<TokenData<Claims>, Error> {
    debug!("Decoding JWT token");
    decode(
        &jwt_token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map(|data: TokenData<Claims>| {
        info!(
            "JWT token decoded successfully for user: {}",
            data.claims.username
        );
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
        Some(header) => header.to_str().map_err(|_| {
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
    let (_bearer, token) = (header.next(), header.next());
    
    // Safely handle the token extraction
    let token = match token {
        Some(t) => t.to_string(),
        None => {
            warn!("Malformed authorization header - missing token");
            return Err(AppError::unauthorized("Authorization header must be 'Bearer <token>'"));
        }
    };
    
    let token_data = match decode_jwt(&token, &state.jwt_secret) {
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

    debug!(
        "Checking membership for user {} in chat {}",
        current_user.user_id, chat_id
    );

    // 3. Verificare che l'utente sia membro della chat tramite metadata
    let metadata = state
        .meta
        .read(&(current_user.user_id, chat_id))
        .await?
        .ok_or_else(|| {
            warn!(
                "User {} is not a member of chat {}",
                current_user.user_id, chat_id
            );
            AppError::forbidden("You are not a member of this chat")
        })?;

    info!(
        "User {} verified as member of chat {}",
        current_user.user_id, chat_id
    );

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
    debug!(
        "Checking role requirements for user {} in chat {}",
        metadata.user_id, metadata.chat_id
    );
    let user_role = metadata.user_role.as_ref().ok_or_else(|| {
        warn!(
            "User role not found in metadata for user {}",
            metadata.user_id
        );
        AppError::forbidden("User role not found in metadata")
    })?;

    if !allowed_roles.contains(user_role) {
        warn!(
            "User {} has insufficient role {:?}, required one of: {:?}",
            metadata.user_id, user_role, allowed_roles
        );
        return Err(
            AppError::forbidden("Insufficient role").with_details(format!(
                "This action requires one of the following roles: {:?}",
                allowed_roles
            )),
        );
    }

    info!(
        "Role check passed for user {} with role {:?}",
        metadata.user_id, user_role
    );
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use axum::{body::Body, http};
    use tokio;
    use crate::entities::UserChatMetadata;

    #[test]
    fn test_encode_decode() {
        let username: String = "UtenteProva1".to_string();
        let id: i32 = 32456;
        let secret: String = "SegretoBellissimo".to_string();

        let encoded = encode_jwt(&username, id, &secret).expect("Encoding JWT must succeed");
        let decoded = decode_jwt(&encoded, &secret).expect("Decoding JWT must succeed");

        // Compare the claims inside the decoded token
        assert_eq!(
            username, decoded.claims.username,
            "Decoded username from JWT must be the same before encoding."
        );
        assert_eq!(
            id, decoded.claims.id,
            "Decoded id from JWT must be the same before encoding."
        );
    }


    #[test]
    fn test_require_role_allows_when_role_present() {
        let metadata = UserChatMetadata {
            user_id: 1,
            chat_id: 10,
            user_role: Some(UserRole::Admin),
            member_since: Utc::now(),
            messages_visible_from: Utc::now(),
            messages_received_until: Utc::now(),
        };

        let allowed_roles = [UserRole::Admin];
        let res = require_role(&metadata, &allowed_roles);
        assert!(
            res.is_ok(),
            "Admin role should be allowed when Admin is in allowed_roles"
        );
    }

    #[test]
    fn test_require_role_denies_when_insufficient() {
        let metadata = UserChatMetadata {
            user_id: 2,
            chat_id: 20,
            user_role: Some(UserRole::Member),
            member_since: Utc::now(),
            messages_visible_from: Utc::now(),
            messages_received_until: Utc::now(),
        };

        let allowed_roles = [UserRole::Admin];
        let res = require_role(&metadata, &allowed_roles);
        assert!(
            res.is_err(),
            "Member role should be denied when only Admin is allowed"
        );
    }

    #[test]
    fn test_require_role_missing_role_field() {
        let metadata = UserChatMetadata {
            user_id: 3,
            chat_id: 30,
            user_role: None,
            member_since: Utc::now(),
            messages_visible_from: Utc::now(),
            messages_received_until: Utc::now(),
        };

        let allowed_roles = [UserRole::Admin];
        let res = require_role(&metadata, &allowed_roles);
        assert!(
            res.is_err(),
            "Missing user_role should result in forbidden error"
        );
    }

    #[tokio::test]
    async fn test_authentication_middleware_header_and_decode_flow() {
        // This test focuses on the header parsing + jwt decode flow that
        // authentication_middleware performs (without relying on DB/AppState).
        let username: String = "MiddlewareUser".to_string();
        let id: i32 = 777;
        let secret: String = "TestSecretForMiddleware".to_string();

        // Create token
        let token = encode_jwt(&username, id, &secret).expect("Encoding JWT must succeed");

        // Build a request with Authorization header like the middleware expects
        let req = Request::builder()
            .uri("/chats/123")
            .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .body(Body::empty())
            .expect("request must build");

        // Extract and parse the header similarly to the middleware
        let auth_header = req
            .headers()
            .get(http::header::AUTHORIZATION)
            .expect("Authorization header must be present")
            .to_str()
            .expect("Header must be valid str");

        let mut header_parts = auth_header.split_whitespace();
        let bearer = header_parts.next();
        let token_str = header_parts.next();

        assert_eq!(
            bearer,
            Some("Bearer"),
            "Authorization scheme must be Bearer"
        );
        let token_str = token_str.expect("token must follow Bearer");

        // Decode token using module function
        let token_data = decode_jwt(&token_str.to_string(), &secret)
            .expect("Decoding JWT must succeed in happy path");

        assert_eq!(token_data.claims.username, username);
        assert_eq!(token_data.claims.id, id);
    }
}
