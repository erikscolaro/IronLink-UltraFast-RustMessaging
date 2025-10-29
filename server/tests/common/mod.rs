use axum_test::TestServer;
use server::core::AppState;
use sqlx::MySqlPool;
use std::sync::Arc;

/// Crea un AppState per i test
///
/// # Arguments
/// * `pool` - Connection pool MySQL
///
/// # Returns
/// Arc<AppState> configurato con il JWT secret di test
pub fn create_test_state(pool: MySqlPool) -> Arc<AppState> {
    let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
    Arc::new(AppState::new(pool, jwt_secret.to_string()))
}

/// Crea un TestServer per i test
///
/// # Arguments
/// * `state` - AppState da utilizzare per il server
///
/// # Returns
/// TestServer configurato e pronto per eseguire richieste
pub fn create_test_server(state: Arc<AppState>) -> TestServer {
    let app = server::create_router(state);
    TestServer::new(app).expect("Failed to create test server")
}

/// Genera un JWT token per testing
///
/// # Arguments
/// * `user_id` - ID dell'utente per cui generare il token
/// * `username` - Username dell'utente
/// * `jwt_secret` - Secret key per firmare il token
///
/// # Returns
/// Token JWT valido per 24 ore
pub fn create_test_jwt(user_id: i32, username: &str, jwt_secret: &str) -> String {
    use chrono::{Duration, Utc};
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        id: i32,
        username: String,
        exp: usize,
        iat: usize,
    }

    let now = Utc::now();
    let expiration = now
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        id: user_id,
        username: username.to_string(),
        exp: expiration,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("Failed to create JWT token")
}
