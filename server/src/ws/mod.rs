//! WebSocket Module - Gestione WebSocket per comunicazione real-time
//!
//! Questo modulo gestisce le connessioni WebSocket per la comunicazione in tempo reale
//! tra client e server. Include:
//! - Gestione upgrade HTTP -> WebSocket
//! - Gestione connessioni (split sender/receiver)
//! - Handler per eventi WebSocket (messaggi, inviti)
//! - Utility per broadcasting e invio errori

pub mod chatmap;
pub mod connection;
pub mod event_handlers;
pub mod usermap;

// Re-exports pubblici
pub use connection::handle_socket;

use crate::{AppState, entities::User};
use axum::{
    Extension,
    extract::{State, ws::WebSocketUpgrade},
    response::Response,
};
use std::sync::Arc;
use tracing::{info, instrument};

// how many messages should the channel contain?
const BROADCAST_CHANNEL_CAPACITY: usize = 100;

/// Intervallo massimo tra invii batch (ms)
const BATCH_INTERVAL: u64 = 1000;

/// Numero massimo di messaggi per batch
const BATCH_MAX_SIZE: usize = 10;

/// Delay minimo tra messaggi client (ms) - max 100 msg/sec
const RATE_LIMITER_MILLIS: u64 = 10;

/// Timeout inattività prima di chiudere connessione (secondi)
const TIMEOUT_DURATION_SECONDS: u64 = 300;

/// Entry point per gestire richieste di upgrade WebSocket
/// Operazioni:
/// 1. Estrarre user_id dall'autenticazione JWT
/// 2. Eseguire upgrade HTTP -> WebSocket
/// 3. Passare la connessione ad handle_socket
#[instrument(skip(ws, state, current_user), fields(user_id = current_user.user_id))]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione JWT
) -> Response {
    let user_id = current_user.user_id;
    info!("WebSocket upgrade requested");

    // Gestisce automaticamente l'upgrade a WebSocket.
    // Se l'upgrade fallisce, ritorna un errore; altrimenti restituisce la nuova connessione al client.

    ws
        // Possibile limitazione dei buffer, default 128 KB
        //.read_buffer_size(4*1024)
        //.write_buffer_size(16*1024)
        .on_upgrade(move |socket| handle_socket(socket, state, user_id))
}
