//! Core Module - Componenti infrastrutturali dell'applicazione
//!
//! Questo modulo contiene tutti i componenti "core" dell'applicazione:
//! - Autenticazione e JWT
//! - Configurazione
//! - Gestione errori
//! - Stato applicazione

pub mod auth;
pub mod config;
pub mod error;
pub mod state;

// Re-exports per facilitare l'import
pub use auth::{authentication_middleware, encode_jwt, decode_jwt, Claims};
pub use config::Config;
pub use error::AppError;
pub use state::AppState;
