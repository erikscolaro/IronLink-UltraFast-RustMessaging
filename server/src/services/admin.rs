//! Admin services - Gestione operazioni amministrative delle chat

use crate::core::{AppError, AppState};
use crate::entities::{User, UserRole};
use axum::{
    extract::{Json, Path, State},
    Extension,
};
use axum_macros::debug_handler;
use std::sync::Arc;

#[debug_handler]
pub async fn update_member_role(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
    Json(body): Json<UserRole>,
) -> Result<(), AppError> {
    // 1. Estrarre user_id dal path della URL e nuovo ruolo dal body JSON
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare il chat_id dal contesto (mancante nel path, da aggiungere alla signature)
    // 4. Recuperare in parallelo i metadata di current_user e target_user per questa chat (implementare una nuova query nel repo find_multiple con WHERE IN)
    // 5. Verificare che entrambi siano membri della chat, altrimenti ritornare errore appropriato
    // 6. Verificare che current_user sia Admin o Owner, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 7. Verificare le regole di promozione: Owner può modificare tutti, Admin può modificare solo Standard (controllo in memoria)
    // 8. Se le regole non sono rispettate, ritornare errore FORBIDDEN
    // 9. Aggiornare il campo user_role nei metadata dell'utente target
    // 10. Creare un messaggio di sistema che notifica il cambio di ruolo
    // 11. Salvare il messaggio nel database
    // 12. Inviare il messaggio tramite WebSocket a tutti i membri online (operazione non bloccante)
    // 13. Ritornare StatusCode::OK
    todo!()
}

pub async fn transfer_ownership(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare l'user_id del nuovo owner dal body della richiesta (mancante nella signature, da aggiungere)
    // 4. Recuperare in parallelo i metadata di current_user e nuovo_owner per questa chat (2 query parallele o singola WHERE IN)
    // 5. Verificare che current_user sia Owner della chat, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 6. Verificare che il nuovo owner sia membro della chat, altrimenti ritornare errore BAD_REQUEST
    // 7. Aggiornare i metadata di entrambi gli utenti in transazione: current_user diventa Admin, nuovo_owner diventa Owner
    // 8. Creare un messaggio di sistema che notifica il trasferimento di ownership
    // 9. Salvare il messaggio nel database
    // 10. Inviare il messaggio tramite WebSocket a tutti i membri online (operazione non bloccante)
    // 11. Ritornare StatusCode::OK
    todo!()
}
