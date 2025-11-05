use axum_test::TestServer;
use server::core::AppState;
use sqlx::MySqlPool;
use std::sync::Arc;
use serde_json::json;

/// Crea un AppState per i test
///
/// # Arguments
/// * `pool` - Reference al connection pool MySQL
///
/// # Returns
/// Arc<AppState> configurato con il JWT secret di test
#[allow(dead_code)]
pub fn create_test_state(pool: &MySqlPool) -> Arc<AppState> {
    let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
    Arc::new(AppState::new(pool.clone(), jwt_secret.to_string()))
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


#[allow(dead_code)]
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


/// Utility per creare solo il token JWT senza connessione WebSocket
///
/// # Arguments
/// * `server` - TestServer da utilizzare
/// * `username` - Username per la registrazione/login
/// * `password` - Password per la registrazione/login
///
/// # Returns
/// Token JWT come String


#[allow(dead_code)]
pub async fn get_auth_token(
    server: &TestServer,
    username: &str,
    password: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Registra l'utente
    let register_body = json!({
        "username": username,
        "password": password
    });
    
    let register_response = server.post("/auth/register").json(&register_body).await;
    register_response.assert_status_ok();

    // 2. Effettua il login per ottenere il token
    let login_body = json!({
        "username": username,
        "password": password
    });

    let login_response = server.post("/auth/login").json(&login_body).await;
    login_response.assert_status_ok();

    // 3. Estrai il token dall'header Authorization
    let auth_header = login_response
        .headers()
        .get("authorization")
        .expect("Authorization header should be present")
        .to_str()
        .expect("Authorization header should be valid string");
    
    let token = auth_header
        .strip_prefix("Bearer ")
        .expect("Authorization should start with 'Bearer '")
        .to_string();

    Ok(token)
}
    
