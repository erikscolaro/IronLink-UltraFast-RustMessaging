//! Chat services - Gestione operazioni sulle chat

use crate::core::{AppError, AppState};
use crate::dtos::{ChatDTO, CreateChatDTO, CreateUserChatMetadataDTO, MessageDTO};
use crate::entities::{Chat, ChatType, User, UserRole};
use crate::repositories::{Create, Read};
use axum::{
    Extension,
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_macros::debug_handler;
use chrono::Utc;
use futures_util::future::try_join_all;
use std::sync::Arc;
use validator::Validate;

/// DTO per creare una chat (estende CreateChatDTO con user_list per chat private)
#[derive(serde::Deserialize)]
pub struct CreateChatRequestDTO {
    pub title: Option<String>,
    pub description: Option<String>,
    pub chat_type: ChatType,
    pub user_list: Option<Vec<i32>>, // Solo per chat private
}

pub async fn list_chats(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>,
) -> Result<Json<Vec<ChatDTO>>, AppError> {
    // 1. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 2. Recuperare tutti i metadata dell'utente dal database tramite user_id (singola query, da implementare)
    // 3. Estrarre tutti i chat_id dai metadata trovati
    // 4. Recuperare tutte le chat in una singola query batch (WHERE chat_id IN (...)) invece di query multiple. chiamarlo find_multiple
    // 5. Convertire ogni Chat in ChatDTO (trasformazione in memoria, nessun I/O)
    // 6. Ritornare la lista di ChatDTO come risposta JSON
    let chat_ids: Vec<i32> = state
        .meta
        .find_all_by_user_id(&current_user.user_id)
        .await?
        .into_iter()
        .map(|s| s.chat_id)
        .collect();

    let chats: Vec<Chat> = try_join_all(chat_ids.into_iter().map(|cid| {
        let state = state.clone();
        async move { state.chat.read(&cid).await }
    }))
    .await?
    .into_iter()
    .flatten()
    .collect();

    let chats_dto: Vec<ChatDTO> = chats.into_iter().map(ChatDTO::from).collect();

    Ok(Json(chats_dto))
}

#[debug_handler]
pub async fn create_chat(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
    Json(body): Json<CreateChatRequestDTO>,
) -> Result<Json<ChatDTO>, AppError> {
    // CASO ChatType::Private:
    // 1. Verificare che user_list sia presente nel body, altrimenti errore BAD_REQUEST
    // 2. Verificare che user_list contenga esattamente 2 utenti, altrimenti errore BAD_REQUEST
    // 3. Verificare che current_user sia uno dei due utenti, altrimenti errore BAD_REQUEST
    // 4. Identificare l'user_id del secondo utente (diverso da current_user)
    // 5. Cercare se esiste già una chat privata tra i due utenti (query DB solo dopo validazioni)
    // 6. Se esiste già, ritornare errore CONFLICT
    // 7. Creare ChatCreateDTO con title=None, description=None, chat_type=Private
    // 8. Salvare la chat nel database (la chiave primaria è autoincrementale)
    // 9. Creare metadata per entrambi gli utenti con ruolo Member e timestamp correnti (preparazione in memoria)
    // 10. Salvare entrambi i metadata nel database in batch/transazione
    //
    // CASO ChatType::Group:
    // 1. Creare ChatCreateDTO con title e description dal body, chat_type=Group
    // 2. Salvare la chat nel database (la chiave primaria è autoincrementale)
    // 3. Creare metadata per current_user con ruolo Owner e timestamp correnti
    // 4. Salvare il metadata nel database
    //
    // FINALE:
    // 1. Convertire la chat creata in ChatDTO (trasformazione in memoria)
    // 2. Ritornare il ChatDTO come risposta JSON
    let chat;
    match body.chat_type {
        ChatType::Private => {
            let user_list = body.user_list.as_ref().ok_or_else(|| {
                AppError::bad_request("Private chat should specify user list.")
            })?;

            if user_list.len() != 2 {
                return Err(AppError::bad_request("Private chat should specify exactly two users."));
            }

            let second_user_id = user_list
                .iter()
                .find(|&&id| id != current_user.user_id)
                .ok_or_else(|| {
                    AppError::bad_request("Current user must be one of the two users.")
                })?;

            let existing_chat = state
                .chat
                .get_private_chat_between_users(&current_user.user_id, second_user_id)
                .await?;
            if existing_chat.is_some() {
                return Err(AppError::conflict("A private chat between these users already exists."));
            }
            let new_chat = CreateChatDTO {
                title: None,
                description: None,
                chat_type: ChatType::Private,
            };
            chat = state.chat.create(&new_chat).await?;

            let now = Utc::now();
            let metadata_current_user = CreateUserChatMetadataDTO {
                user_id: current_user.user_id,
                chat_id: chat.chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            };

            let metadata_second_user = CreateUserChatMetadataDTO {
                user_id: *second_user_id,
                chat_id: chat.chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            };
            state.meta.create(&metadata_current_user).await?;
            state.meta.create(&metadata_second_user).await?;
        }

        ChatType::Group => {
            let new_chat = CreateChatDTO {
                title: body.title.clone(),
                description: body.description.clone(),
                chat_type: ChatType::Group,
            };

            // Validazione con validator
            new_chat.validate()?;

            chat = state.chat.create(&new_chat).await?;

            let now = Utc::now();
            let metadata_owner = CreateUserChatMetadataDTO {
                user_id: current_user.user_id,
                chat_id: chat.chat_id,
                user_role: Some(UserRole::Owner),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            };

            state.meta.create(&metadata_owner).await?;
        }
    }

    let chat_dto = ChatDTO::from(chat);
    Ok(Json(chat_dto))
}

pub async fn get_chat_messages(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    // Query(params): Query<QueryStructCursom>,   // da vedere, se conviene o meno
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<Json<Vec<MessageDTO>>, AppError> {
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare il metadata dell'utente per questa chat (singola query che fa sia controllo membership che recupero timestamp)
    // 4. Se metadata non esiste (utente non membro), ritornare errore FORBIDDEN
    // 5. Recuperare tutti i messaggi della chat filtrati per timestamp >= messages_visible_from in una singola query
    // 6. Convertire ogni messaggio in MessageDTO (trasformazione in memoria, nessun I/O)
    // 7. Ritornare la lista di MessageDTO come risposta JSON
    // Nota: paginazione (limit, offset) da implementare in futuro tramite Query params
    todo!()
}
