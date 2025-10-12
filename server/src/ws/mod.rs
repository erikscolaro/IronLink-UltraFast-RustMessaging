//! WebSocket Module - Gestione WebSocket per comunicazione real-time
//!
//! Questo modulo gestisce le connessioni WebSocket per la comunicazione in tempo reale
//! tra client e server. Include:
//! - Gestione upgrade HTTP -> WebSocket
//! - Gestione connessioni (split sender/receiver)
//! - Handler per eventi WebSocket (messaggi, inviti)
//! - Utility per broadcasting e invio errori

pub mod connection;
pub mod event_handlers;
pub mod utils;

// Re-exports pubblici
pub use connection::handle_socket;
pub use utils::{broadcast_to_chat, send_error_to_user};

use crate::{entities::User, AppState};
use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    Extension,
};
use std::sync::Arc;

/// Entry point per gestire richieste di upgrade WebSocket
/// Operazioni:
/// 1. Estrarre user_id dall'autenticazione JWT
/// 2. Eseguire upgrade HTTP -> WebSocket
/// 3. Passare la connessione ad handle_socket
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione JWT
) -> Response {
    let user_id = current_user.user_id;

    // Gestisce automaticamente l'upgrade a WebSocket.
    // Se l'upgrade fallisce, ritorna un errore; altrimenti restituisce la nuova connessione al client.
    ws
        // Possibile limitazione dei buffer, default 128 KB
        //.read_buffer_size(4*1024)
        //.write_buffer_size(16*1024)
        .on_upgrade(move |socket| handle_socket(socket, state, user_id))
}
