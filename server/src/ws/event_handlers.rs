//! WebSocket Event Handlers - Handler per eventi WebSocket

use crate::dtos::{InvitationDTO, MessageDTO};
use crate::AppState;
use std::sync::Arc;

/// Handler per messaggi di chat
/// Operazioni:
/// 1. Validare il messaggio (chat esiste? utente è membro?)
/// 2. Salvare il messaggio nel database
/// 3. Inoltrare il messaggio a tutti i membri online della chat
pub async fn process_chat_message(_state: Arc<AppState>, _user_id: i32, _event: MessageDTO) {
    /*
    - controllare se la chat con l'utente esiste
    - salvare a db il messaggio
     */

    /*
    state.user_online.get(event.destinatario) => tx (se online) altrimenti none
    se online => tx.send(event)
     */

    todo!()
}

/// Handler per inviti a chat
/// Operazioni:
/// 1. Validare l'invito (chat esiste? utente ha permessi di invitare?)
/// 2. Salvare l'invito nel database
/// 3. Notificare l'utente invitato se è online
pub async fn process_invitation(_state: Arc<AppState>, _user_id: i32, _event: InvitationDTO) {
    todo!()
}
