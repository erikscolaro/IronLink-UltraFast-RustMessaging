//! WebSocket Utilities - Funzioni di supporto per WebSocket

use crate::AppState;
use crate::dtos::WsEventDTO;

/// Invia un messaggio di errore ad un utente specifico
/// Operazioni:
/// 1. Verificare se l'utente è online
/// 2. Inviare WsEventDTO::Error sul suo canale
/// 3. Gestire errore se l'utente si è disconnesso
pub async fn send_error_to_user(state: &AppState, user_id: i32, error_code: u16, message: String) {
    if let Some(tx) = state.users_online.get(&user_id) {
        // invio direttamente il WsEventDTO sul canale
        if tx
            .send(WsEventDTO::Error {
                code: error_code,
                message,
            })
            .await
            .is_err()
        {
            if cfg!(debug_assertions) {
                eprintln!(
                    "Client disconnesso, errore non inviato all'utente {}",
                    user_id
                );
            }
        }
    }
}

/// Invia un evento a tutti i membri online di una chat
/// Operazioni:
/// 1. Recuperare lista membri della chat dal database
/// 2. Per ogni membro, verificare se è online
/// 3. Inviare l'evento al canale di ciascun membro online
/// 4. Gestire errori di invio (utenti disconnessi)
pub async fn broadcast_to_chat(
    state: &AppState,
    chat_id: i32,
    event: WsEventDTO,
) -> Result<usize, sqlx::Error> {
    // 1. Recuperare i membri della chat
    let members = state.meta.get_members_by_chat(&chat_id).await?;

    let mut sent_count = 0;

    // 2. Per ogni membro, inviare l'evento se è online
    for member in &members {
        if let Some(tx) = state.users_online.get(&member.user_id) {
            // Cloniamo l'evento per ogni destinatario
            if tx.send(event.clone()).await.is_ok() {
                sent_count += 1;
            } else if cfg!(debug_assertions) {
                eprintln!(
                    "Impossibile inviare a utente {} (disconnesso)",
                    member.user_id
                );
            }
        }
    }

    if cfg!(debug_assertions) {
        println!(
            "Broadcast a chat {}: {} membri online raggiunti su {} totali",
            chat_id,
            sent_count,
            members.len()
        );
    }

    Ok(sent_count)
}
