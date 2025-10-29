//! Chat services - Gestione operazioni sulle chat

use crate::core::{AppError, AppState};
use crate::dtos::{ChatDTO, CreateChatDTO, CreateUserChatMetadataDTO, MessageDTO, MessagesQuery};
use crate::entities::{Chat, ChatType, User, UserChatMetadata, UserRole};
use crate::repositories::{Create, Read};
use axum::{
    Extension,
    extract::{Json, Path, Query, State},
};
use chrono::Utc;
use futures_util::future::try_join_all;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};
use validator::Validate;

/// DTO per creare una chat (estende CreateChatDTO con user_list per chat private)
#[derive(serde::Deserialize)]
pub struct CreateChatRequestDTO {
    pub title: Option<String>,
    pub description: Option<String>,
    pub chat_type: ChatType,
    pub user_list: Option<Vec<i32>>, // Solo per chat private
}

#[instrument(skip(state, current_user), fields(user_id = %current_user.user_id))]
pub async fn list_chats(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>,
) -> Result<Json<Vec<ChatDTO>>, AppError> {
    debug!("Listing chats for user");
    // 1. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 2. Recuperare tutti i metadata dell'utente dal database tramite user_id (singola query)
    // 3. Estrarre tutti i chat_id dai metadata trovati
    // 4. Recuperare tutte le chat con query parallele (primary key lookup, velocissimo)
    // 5. Convertire ogni Chat in ChatDTO (trasformazione in memoria, nessun I/O)
    // 6. Ritornare la lista di ChatDTO come risposta JSON
    let chat_ids: Vec<i32> = state
        .meta
        .find_many_by_user_id(&current_user.user_id)
        .await?
        .into_iter()
        .map(|s| s.chat_id)
        .collect();

    debug!("User is member of {} chats", chat_ids.len());

    let chats: Vec<Chat> = try_join_all(chat_ids.into_iter().map(|cid| {
        let state = state.clone();
        async move { state.chat.read(&cid).await }
    }))
    .await?
    .into_iter()
    .flatten()
    .collect();

    let chats_dto: Vec<ChatDTO> = chats.into_iter().map(ChatDTO::from).collect();

    info!("Successfully retrieved {} chats", chats_dto.len());
    Ok(Json(chats_dto))
}

#[instrument(skip(state, current_user, body), fields(user_id = %current_user.user_id, chat_type = ?body.chat_type))]
pub async fn create_chat(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
    Json(body): Json<CreateChatRequestDTO>,
) -> Result<Json<ChatDTO>, AppError> {
    debug!("Creating new chat");
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
            debug!("Creating private chat");
            let user_list = body.user_list.as_ref().ok_or_else(|| {
                warn!("Private chat creation attempted without user list");
                AppError::bad_request("Private chat should specify user list.")
            })?;

            if user_list.len() != 2 {
                warn!(
                    "Private chat creation attempted with {} users instead of 2",
                    user_list.len()
                );
                return Err(AppError::bad_request(
                    "Private chat should specify exactly two users.",
                ));
            }

            let second_user_id = user_list
                .iter()
                .find(|&&id| id != current_user.user_id)
                .ok_or_else(|| {
                    warn!("Current user not in user list for private chat");
                    AppError::bad_request("Current user must be one of the two users.")
                })?;

            let existing_chat = state
                .chat
                .get_private_chat_between_users(&current_user.user_id, second_user_id)
                .await?;
            if existing_chat.is_some() {
                warn!(
                    "Private chat already exists between users {} and {}",
                    current_user.user_id, second_user_id
                );
                return Err(AppError::conflict(
                    "A private chat between these users already exists.",
                ));
            }
            let new_chat = CreateChatDTO {
                title: None,
                description: None,
                chat_type: ChatType::Private,
            };
            chat = state.chat.create(&new_chat).await?;

            debug!("Private chat created with id {}", chat.chat_id);

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

            // Create both metadata in a single transaction for atomicity
            state
                .meta
                .create_many(&[metadata_current_user, metadata_second_user])
                .await?;

            info!(
                "Private chat created successfully between users {} and {}",
                current_user.user_id, second_user_id
            );
        }

        ChatType::Group => {
            debug!("Creating group chat");
            let new_chat = CreateChatDTO {
                title: body.title.clone(),
                description: body.description.clone(),
                chat_type: ChatType::Group,
            };

            // Validazione con validator
            new_chat.validate()?;

            chat = state.chat.create(&new_chat).await?;

            debug!("Group chat created with id {}", chat.chat_id);

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

            info!(
                "Group chat '{}' created successfully by user {}",
                chat.title.as_ref().unwrap_or(&String::from("Unnamed")),
                current_user.user_id
            );
        }
    }

    let chat_dto = ChatDTO::from(chat);
    Ok(Json(chat_dto))
}

#[instrument(skip(state, metadata), fields(chat_id = %chat_id, user_id = %metadata.user_id))]
pub async fn get_chat_messages(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Query(params): Query<MessagesQuery>,
    Extension(metadata): Extension<UserChatMetadata>, // ottenuto dal chat_membership_middleware
) -> Result<Json<Vec<MessageDTO>>, AppError> {
    debug!("Fetching chat messages");
    // 1. Estrarre chat_id dal path della URL
    // 2. Estrarre query parameters (before_date opzionale)
    // 3. Ottenere metadata dell'utente dall'Extension (inserito dal chat_membership_middleware)
    // 4. Se before_date presente: recuperare 50 messaggi prima di quella data
    //    Altrimenti: recuperare ultimi 50 messaggi disponibili
    // 5. Convertire ogni messaggio in MessageDTO (trasformazione in memoria, nessun I/O)
    // 6. Ritornare la lista di MessageDTO come risposta JSON

    let (before_date, limit) = if let Some(date) = params.before_date {
        (Some(date), 50)
    } else {
        (None, 50)
    };

    let messages = state
        .msg
        .find_many_paginated(
            &chat_id,
            &metadata.messages_visible_from,
            before_date.as_ref(),
            limit,
        )
        .await?;

    info!("Retrieved {} messages for chat", messages.len());

    let messages_dto: Vec<MessageDTO> = messages.into_iter().map(MessageDTO::from).collect();

    Ok(Json(messages_dto))
}
