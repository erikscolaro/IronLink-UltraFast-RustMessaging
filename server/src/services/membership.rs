//! Membership services - Gestione membri e ruoli nelle chat

use crate::core::{AppError, AppState};
use crate::dtos::{CreateChatDTO, CreateMessageDTO, CreateUserChatMetadataDTO, MessageDTO, UserInChatDTO, WsEventDTO};
use crate::entities::{ChatType, User, UserRole, MessageType};
use crate::repositories::{Create, Read};
use axum::{
    Extension,
    extract::{Json, Path, State},
};
use axum_macros::debug_handler;
use std::sync::Arc;
use chrono::Utc;

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

    let var: Vec<_> = user_ids.iter().map(|id| state.user.read(id)).collect();
    let results: Vec<Option<User>> = futures::future::try_join_all(var).await?;
    let users: Vec<User> = results.into_iter().filter_map(|u| u).collect();

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

    let meta_opt = state
        .meta
        .find_by_user_and_chat_id(&current_user.user_id, &chat_id)
        .await?;

    let meta = match meta_opt {
        Some(m) => m,
        None => {
            return Err(AppError::forbidden(
                "You are not a member of this chat".to_string(),
                ));
            }
    };

    match meta.user_role {
        Some(UserRole::Member) => {},
        _ => {
            return Err(AppError::forbidden(
                "You do not have permission to invite users to this chat".to_string(),
            ));
        }
    }

    let is_present = state
        .meta
        .find_by_user_and_chat_id(&user_id, &chat_id)
        .await?
        .is_some();
    if is_present {
        return Err(AppError::conflict(
            "User is already a member of this chat".to_string(),
        ));
    }

    let has_pending_invite = state
        .invitation
        .has_pending_invitation(&user_id, &chat_id)
        .await?;
    if has_pending_invite {
        return Err(AppError::conflict(
            "There is already a pending invitation for this user to this chat".to_string(),
        ));
    }

    let user = state
        .user
        .read(&user_id)
        .await?;
    if user.is_none() {
        return Err(AppError::not_found("User not found".to_string()));
    }

    let chat = state
        .chat
        .get_private_chat_between_users(&current_user.user_id, &user_id)
        .await?;

    let final_chat_id = if let Some(existing_chat) = chat {
        existing_chat.chat_id
    } else {
        let new_chat_dto = CreateChatDTO {
            title: None,
            description: None,
            chat_type: ChatType::Private,
        };

        let new_chat = state
            .chat
            .create(&new_chat_dto)
            .await?;

        let now = Utc::now();
            let metadata_current_user = CreateUserChatMetadataDTO {
                user_id: current_user.user_id,
                chat_id: new_chat.chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            };

            let metadata_second_user = CreateUserChatMetadataDTO {
                user_id: user_id,
                chat_id: new_chat.chat_id,
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
        new_chat.chat_id
    };

    let create_message_dto = CreateMessageDTO {
        chat_id: final_chat_id,
        sender_id: current_user.user_id,
        content: format!("User {} has invited you to the chat", current_user.username),
        message_type: MessageType::SystemMessage, 
        created_at: Utc::now(), 
    };

    let _saved_message = state
        .msg
        .create(&create_message_dto)
        .await?;

    let message_dto = MessageDTO {
        message_id: None,
        chat_id: Some(create_message_dto.chat_id),
        sender_id: Some(create_message_dto.sender_id),
        content: Some(create_message_dto.content.clone()),
        message_type: Some(create_message_dto.message_type.clone()),
        created_at: Some(create_message_dto.created_at),
    };

    if let Some(sender_ref) = state.users_online.get(&user_id) {
        let sender = sender_ref.clone();
        // costruisco l'evento WS; adattalo se il variant/shape di WsEventDTO è diverso
        let ws_event = WsEventDTO::Message(message_dto.clone());
        tokio::spawn(async move {
            let _ = sender.send(ws_event).await;
        });   
    }

    Ok(())
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

    let meta_opt = state
        .meta
        .find_by_user_and_chat_id(&current_user.user_id, &chat_id)
        .await?;

    let meta = match meta_opt {
        Some(m) => m,
        None => {
            return Err(AppError::not_found(
                "You are not a member of this chat".to_string(),
                ));
            }
    };

    match meta.user_role {
        Some(UserRole::Owner) => {
            return Err(AppError::conflict(
                "The owner cannot leave the chat. Transfer ownership or delete the chat.".to_string(),
            ));
        },
        _ => {}
    }

    let members = state
        .meta
        .find_many_by_chat_id(&chat_id)
        .await?;

    state
        .meta
        .delete_by_user_and_chat_id(&current_user.user_id, &chat_id)
        .await?;

    let create_message_dto = CreateMessageDTO {
        chat_id: chat_id,
        sender_id: current_user.user_id,
        content: format!("User {} has left the chat", current_user.username),
        message_type: MessageType::SystemMessage,
        created_at: Utc::now(),
    };

    let _saved_message = state
        .msg
        .create(&create_message_dto)
        .await?;

    let message_dto = MessageDTO {
        message_id: None,
        chat_id: Some(create_message_dto.chat_id),
        sender_id: Some(create_message_dto.sender_id),
        content: Some(create_message_dto.content.clone()),
        message_type: Some(create_message_dto.message_type.clone()),
        created_at: Some(create_message_dto.created_at),
    };

    for member in members {
        if member.user_id == current_user.user_id {
            continue;
        }

        if let Some(sender_ref) = state.users_online.get(&member.user_id) {
            let sender = sender_ref.clone();
            let ws_event = WsEventDTO::Message(message_dto.clone());
            tokio::spawn(async move {
                let _ = sender.send(ws_event).await;
            });   
        }
    }

    Ok(())
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

    let current_meta_opt = state
        .meta
        .find_by_user_and_chat_id(&current_user.user_id, &chat_id)
        .await?;

    let current_meta = match current_meta_opt {
        Some(m) => m,
        None => {
            return Err(AppError::forbidden(
                "You are not a member of this chat".to_string(),
                ));
            }
    };

    match current_meta.user_role {
        Some(UserRole::Member) => {
            return Err(AppError::forbidden(
                "You do not have permission to remove users from this chat".to_string(),
            ));
        },
        _ => {}
    }

    let target_meta_opt = state
        .meta
        .find_by_user_and_chat_id(&user_id, &chat_id)
        .await?;

    let target_meta = match target_meta_opt {
        Some(m) => m,
        None => {
            return Err(AppError::not_found(
                "The user to be removed is not a member of this chat".to_string(),
                ));
            }
    };

    match target_meta.user_role {
        Some(UserRole::Owner) => {
            return Err(AppError::forbidden(
                "You cannot remove the owner of the chat".to_string(),
            ));
        },
        _ => {}
    }

    let members = state
        .meta
        .find_many_by_chat_id(&chat_id)
        .await?;

    state
        .meta
        .delete_by_user_and_chat_id(&user_id, &chat_id)
        .await?;

    let target_user_opt = state
        .user
        .read(&user_id)
        .await?;

    let target_username = target_user_opt
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "Unknown User".to_string());

    let create_message_dto = CreateMessageDTO {
        chat_id: chat_id,
        sender_id: current_user.user_id,
        content: format!("User {} has removed {} from the chat", current_user.username, target_username),
        message_type: MessageType::SystemMessage,
        created_at: Utc::now(),
    };

    let _saved_message = state
        .msg
        .create(&create_message_dto)
        .await?;

    let message_dto = MessageDTO {
        message_id: None,
        chat_id: Some(create_message_dto.chat_id),
        sender_id: Some(create_message_dto.sender_id),
        content: Some(create_message_dto.content.clone()),
        message_type: Some(create_message_dto.message_type.clone()),
        created_at: Some(create_message_dto.created_at),
    };

    for member in members {
        if let Some(sender_ref) = state.users_online.get(&member.user_id) {
            let sender = sender_ref.clone();
            let ws_event = WsEventDTO::Message(message_dto.clone());
            tokio::spawn(async move {
                let _ = sender.send(ws_event).await;
            });   
        }
    }

    Ok(())
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
