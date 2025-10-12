//! Test helpers and utilities
//!
//! Questo modulo contiene funzioni helper condivise tra tutti i test.
//!
//! ## Setup Database
//! I test usano il macro `#[sqlx::test]` che gestisce automaticamente:
//! - Creazione di un database di test isolato per ogni test
//! - Applicazione automatica delle migrations da `migrations/`
//! - Applicazione opzionale di fixtures da `fixtures/`
//! - Cleanup automatico del database al termine del test
//!
//! ## Environment Variables
//! Richiede `DATABASE_URL` con credenziali superuser (root) per creare/distruggere database di test.
//! Esempio: `DATABASE_URL=mysql://root:password@localhost:3306`

/// Genera un JWT token per testing
///
/// # Arguments
/// * `user_id` - ID dell'utente per cui generare il token
/// * `jwt_secret` - Secret key per firmare il token
///
/// # Returns
/// Token JWT valido per 24 ore
pub fn create_test_jwt(user_id: i32, jwt_secret: &str) -> String {
    use chrono::{Duration, Utc};
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: i64,
    }

    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("Failed to create JWT token")
}
