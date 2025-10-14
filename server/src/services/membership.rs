//! Membership services - Gestione membri e ruoli nelle chat

use crate::core::{AppError, AppState};
use crate::dtos::UserInChatDTO;
use crate::entities::{User, UserRole};
use axum::{
    Extension,
    extract::{Json, Path, State},
};
use axum_macros::debug_handler;
use std::sync::Arc;

pub async fn list_chat_members(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<Json<Vec<UserInChatDTO>>, AppError> {
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare tutti i metadata associati alla chat tramite chat_id (singola query)
    // 4. Verificare se current_user è tra i membri, altrimenti ritornare errore FORBIDDEN (controllo in memoria)
    // 5. Estrarre tutti gli user_id dai metadata
    // 6. Recuperare tutti gli utenti in una singola query batch (WHERE user_id IN (...))
    // 7. Combinare le informazioni degli utenti con i metadata (join in memoria)
    // 8. Convertire ogni combinazione in UserInChatDTO (trasformazione in memoria)
    // 9. Ritornare la lista di UserInChatDTO come risposta JSON
    
    let meta = state
        .meta
        .find_many_by_chat_id(&chat_id)
        .await?;

    let is_member = meta.iter().any(|m| m.user_id == current_user.user_id);
    if !is_member {
        return Err(AppError::forbidden(
            "You are not a member of this chat".to_string(),
        ));
    }

    let user_ids: Vec<i32> = meta.iter().map(|m| m.user_id).collect();

    let users = state
        .user
        .find_many_by_ids(&user_ids)
        .await?;

    let mut result: Vec<UserInChatDTO> = Vec::new();
    for user in users {
        if let Some(m) = meta.iter().find(|m| m.user_id == user.user_id) {
            result.push(UserInChatDTO {
                user_id: Some(user.user_id),
                chat_id: Some(m.chat_id),
                username: Some(user.username),
                user_role: m.user_role.clone(),
                member_since: Some(m.member_since)
            });
        }
    }

    Ok(Json(result))
}

pub async fn invite_to_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Path(user_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // 1. Estrarre chat_id e user_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare il metadata di current_user per questa chat (singola query per controllo permessi)
    // 4. Verificare che current_user sia Admin o Owner, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 5. Verificare che l'utente target non sia già membro della chat (query metadata target)
    // 6. Se è già membro, ritornare errore CONFLICT
    // 7. Controllare se esiste già un invito pending per questo utente in questa chat
    // 8. Se esiste già un invito pending, ritornare errore CONFLICT
    // 9. Verificare che l'utente target esista nel database (query solo se tutte le validazioni passano)
    // 10. Se non esiste, ritornare errore NOT_FOUND
    // 11. Creare o recuperare una chat privata tra current_user e l'utente target
    // 12. Creare un messaggio di sistema con l'invito alla chat
    // 13. Salvare il messaggio di invito nel database
    // 14. Inviare il messaggio tramite WebSocket all'utente target se online (operazione non bloccante)
    // 15. Ritornare StatusCode::OK
    todo!()
}

pub async fn leave_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare il metadata di current_user per questa chat (singola query)
    // 4. Se metadata non esiste (non membro), ritornare errore NOT_FOUND (fail-fast)
    // 5. Verificare il ruolo: se è Owner, ritornare errore CONFLICT con messaggio specifico (fail-fast, controllo in memoria)
    // 6. Cancellare i metadata di current_user per questa chat dal database
    // 7. Creare un messaggio di sistema che notifica l'uscita (i messaggi dell'utente rimangono nel DB)
    // 8. Salvare il messaggio nel database
    // 9. Inviare il messaggio tramite WebSocket a tutti i membri online (operazione non bloccante)
    // 10. Ritornare StatusCode::OK
    todo!()
}

pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Path(user_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // 1. Estrarre chat_id e user_id dal path dalla URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare in parallelo i metadata di current_user e target_user per questa chat (implementare una nuova query nel repo find_multiple con WHERE IN)
    // 4. Verificare che current_user sia Admin o Owner, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 5. Verificare che l'utente target sia membro della chat, altrimenti ritornare errore NOT_FOUND
    // 6. Verificare che non si stia cercando di rimuovere l'Owner, altrimenti ritornare errore FORBIDDEN (controllo in memoria)
    // 7. Cancellare i metadata dell'utente target per questa chat dal database
    // 8. Creare un messaggio di sistema che notifica la rimozione del membro (i messaggi dell'utente rimangono nel DB)
    // 9. Salvare il messaggio nel database
    // 10. Inviare il messaggio tramite WebSocket a tutti i membri online incluso il rimosso (operazione non bloccante)
    // 11. Ritornare StatusCode::OK
    todo!()
}

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
    // 7. Verificare le regole di promozione: Owner può modificare tutti, Admin può modificare solo Member (controllo in memoria)
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
