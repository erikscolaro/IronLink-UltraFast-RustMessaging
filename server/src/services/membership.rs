//! Membership services - Gestione membri e ruoli nelle chat

use crate::core::{AppError, AppState, require_role};
use crate::dtos::{
    CreateInvitationDTO, CreateMessageDTO, CreateUserChatMetadataDTO, InvitationDTO, MessageDTO,
    UpdateInvitationDTO, UserInChatDTO,
};
use crate::entities::{ChatType, InvitationStatus, MessageType, User, UserChatMetadata, UserRole};
use crate::repositories::{Create, Delete, Read, Update};
use crate::ws::usermap::InternalSignal;
use axum::{
    Extension,
    extract::{Json, Path, State},
};
use axum_macros::debug_handler;
use chrono::Utc;
use std::sync::Arc;
use validator::Validate;
use tracing::{debug, info, instrument, warn};

#[instrument(skip(state, _metadata), fields(chat_id = %chat_id))]
pub async fn list_chat_members(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Extension(_metadata): Extension<UserChatMetadata>, // ottenuto dal chat_membership_middleware (verifica già la membership)
) -> Result<Json<Vec<UserInChatDTO>>, AppError> {
    debug!("Listing members for chat");
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere metadata dell'utente dall'Extension (membership già verificata dal middleware)
    // 3. Recuperare tutti i metadata associati alla chat tramite chat_id (singola query)
    // 4. Estrarre tutti gli user_id dai metadata
    // 5. Recuperare tutti gli utenti con query parallele per ogni user_id
    // 6. Combinare le informazioni degli utenti con i metadata (join in memoria)
    // 7. Convertire ogni combinazione in UserInChatDTO (trasformazione in memoria)
    // 8. Ritornare la lista di UserInChatDTO come risposta JSON

    let meta = state.meta.find_many_by_chat_id(&chat_id).await?;

    debug!("Found {} members in chat", meta.len());

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
                member_since: Some(m.member_since),
            });
        }
    }

    info!("Successfully retrieved {} members", result.len());
    Ok(Json(result))
}

#[instrument(skip(state, current_user), fields(user_id = %current_user.user_id))]
pub async fn list_pending_invitations(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>,
) -> Result<Json<Vec<InvitationDTO>>, AppError> {
    debug!("Listing pending invitations for user");
    // 1. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 2. Recuperare tutti gli inviti pending per l'utente corrente
    // 3. Convertire ogni invito in InvitationDTO
    // 4. Ritornare la lista di InvitationDTO come risposta JSON

    let invitations = state
        .invitation
        .find_many_by_user_id(&current_user.user_id)
        .await?;

    info!("Found {} pending invitations", invitations.len());

    let invitations_dto: Vec<InvitationDTO> =
        invitations.into_iter().map(InvitationDTO::from).collect();

    Ok(Json(invitations_dto))
}

#[instrument(skip(state, current_user, metadata), fields(chat_id = %chat_id, inviting_user = %current_user.user_id, target_user = %user_id))]
pub async fn invite_to_chat(
    State(state): State<Arc<AppState>>,
    Path((chat_id, user_id)): Path<(i32, i32)>,
    Extension(current_user): Extension<User>,
    Extension(metadata): Extension<UserChatMetadata>, // ottenuto dal chat_membership_middleware
) -> Result<(), AppError> {
    debug!("Inviting user to chat");
    // 1. Estrarre chat_id e user_id dal path, ottenere utente corrente e metadata dall'Extension
    // 2. Verificare che current_user sia Admin o Owner tramite metadata
    // 3. Verificare che la chat esista e sia di tipo Group (non si può invitare in chat private)
    // 4. Verificare che l'utente target esista nel database (fail-fast su controllo basilare)
    // 5. Verificare che l'utente target non sia già membro
    // 6. Controllare se esiste già un invito pending
    // 7. Creare l'invitation nel database
    // 8. Inviare l'invitation via WebSocket all'utente invitato (se online)
    // 9. Ritornare OK

    require_role(&metadata, &[UserRole::Admin, UserRole::Owner])?;

    // Verificare che la chat esista e sia di tipo Group
    let chat = state
        .chat
        .read(&chat_id)
        .await?
        .ok_or_else(|| {
            warn!("Chat not found: {}", chat_id);
            AppError::not_found("Chat not found")
        })?;

    if chat.chat_type != ChatType::Group {
        warn!("Attempted to invite user to private chat");
        return Err(AppError::bad_request("Cannot invite users to private chats"));
    }

    // Verificare che l'utente target esista nel database
    if state.user.read(&user_id).await?.is_none() {
        warn!("Target user not found: {}", user_id);
        return Err(AppError::not_found("User not found"));
    }

    // Verificare che l'utente target non sia già membro
    if state.meta.read(&(user_id, chat_id)).await?.is_some() {
        warn!("User {} is already a member of chat {}", user_id, chat_id);
        return Err(AppError::conflict("User is already a member of this chat"));
    }

    // Controllare se esiste già un invito pending
    if state
        .invitation
        .has_pending_invitation(&user_id, &chat_id)
        .await?
    {
        warn!("Pending invitation already exists for user {} to chat {}", user_id, chat_id);
        return Err(AppError::conflict(
            "There is already a pending invitation for this user to this chat",
        ));
    }

    // Creare l'invitation nel database
    let invitation = state
        .invitation
        .create(&CreateInvitationDTO {
            target_chat_id: chat_id,
            invited_id: user_id,
            invitee_id: current_user.user_id,
        })
        .await?;

    debug!("Invitation created with id {}", invitation.invite_id);

    // Inviare l'invitation via WebSocket all'utente invitato (se online)
    let invitation_dto = InvitationDTO::from(invitation);
    state
        .users_online
        .send_server_message_if_online(&user_id, InternalSignal::Invitation(invitation_dto));

    info!("User successfully invited to chat");
    Ok(())
}

#[instrument(skip(state, current_user), fields(invite_id = %invite_id, action = %action, user_id = %current_user.user_id))]
pub async fn respond_to_invitation(
    State(state): State<Arc<AppState>>,
    Path((invite_id, action)): Path<(i32, String)>,
    Extension(current_user): Extension<User>,
) -> Result<(), AppError> {
    debug!("Responding to invitation");
    // 1. Estrarre invite_id e action (accept/reject) dal path
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Validare che action sia "accept" o "reject"
    // 4. Recuperare l'invito dal database
    // 5. Verificare che l'invito sia pending e che current_user sia l'invitato
    // 6. Se accept: creare metadata per aggiungere l'utente alla chat con ruolo Member
    // 7. Se accept e utente online: inviare segnale AddChat per sottoscriversi ai messaggi
    // 8. Aggiornare lo stato dell'invito (Accepted/Rejected)
    // 9. Creare messaggio di sistema nella chat target con notifica appropriata
    // 10. Salvare il messaggio dopo validazione
    // 11. Inviare il messaggio tramite WebSocket a tutti i membri online
    // 12. Ritornare OK

    // Validare action
    let new_status = match action.as_str() {
        "accept" => InvitationStatus::Accepted,
        "reject" => InvitationStatus::Rejected,
        _ => {
            warn!("Invalid invitation action: {}", action);
            return Err(AppError::bad_request("Action must be 'accept' or 'reject'"));
        }
    };

    // Recuperare l'invito
    let invitation = state
        .invitation
        .read(&invite_id)
        .await?
        .ok_or_else(|| {
            warn!("Invitation not found: {}", invite_id);
            AppError::not_found("Invitation not found")
        })?;

    // Verificare che sia pending
    if invitation.state != InvitationStatus::Pending {
        warn!("Invitation {} is already processed: {:?}", invite_id, invitation.state);
        return Err(AppError::conflict("Invitation is already processed").with_details(format!(
            "Invitation is already {:?}",
            invitation.state
        )));
    }

    // Verificare che current_user sia l'invitato
    if invitation.invited_id != current_user.user_id {
        warn!("User {} attempted to respond to invitation for user {}", current_user.user_id, invitation.invited_id);
        return Err(AppError::forbidden("You are not the recipient of this invitation"));
    }

    let chat_id = invitation.target_chat_id;

    // Se accetta, aggiungere l'utente alla chat
    if matches!(new_status, InvitationStatus::Accepted) {
        debug!("User accepted invitation, adding to chat {}", chat_id);
        let now = Utc::now();
        state
            .meta
            .create(&CreateUserChatMetadataDTO {
                user_id: current_user.user_id,
                chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            })
            .await?;

        // Se l'utente è online, inviare segnale AddChat per sottoscriversi ai messaggi della chat
        state
            .users_online
            .send_server_message_if_online(&current_user.user_id, InternalSignal::AddChat(chat_id));
    } else {
        debug!("User rejected invitation");
    }

    // Aggiornare lo stato dell'invito
    state
        .invitation
        .update(
            &invite_id,
            &UpdateInvitationDTO {
                state: Some(new_status.clone()),
            },
        )
        .await?;

    // Creare messaggio di sistema appropriato
    let content = if matches!(new_status, InvitationStatus::Accepted) {
        format!("User {} has joined the chat", current_user.username)
    } else {
        format!("User {} has declined the invitation", current_user.username)
    };

    let create_dto = CreateMessageDTO {
        chat_id,
        sender_id: current_user.user_id,
        content,
        message_type: MessageType::SystemMessage,
        created_at: Utc::now(),
    };

    create_dto
        .validate()
        .map_err(|_| AppError::bad_request("Validation error"))?;

    let saved_message = state.msg.create(&create_dto).await?;

    let _ = state
        .chats_online
        .send(&chat_id, Arc::new(MessageDTO::from(saved_message)));
    
    info!("Invitation response processed successfully");
    Ok(())
}

#[instrument(skip(state, current_user, metadata), fields(chat_id = %chat_id, user_id = %current_user.user_id))]
pub async fn leave_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Extension(current_user): Extension<User>,
    Extension(metadata): Extension<UserChatMetadata>, // ottenuto dal chat_membership_middleware
) -> Result<(), AppError> {
    debug!("User leaving chat");
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere l'utente corrente e metadata dall'Extension
    // 3. Verificare il ruolo: se è Owner, ritornare errore CONFLICT con messaggio specifico (fail-fast, controllo in memoria)
    // 4. Cancellare i metadata di current_user per questa chat dal database
    // 5. Se utente online: inviare segnale RemoveChat per disiscriversi dai messaggi della chat
    // 6. Creare un messaggio di sistema che notifica l'uscita (i messaggi dell'utente rimangono nel DB)
    // 7. Salvare il messaggio nel database
    // 8. Inviare il messaggio tramite WebSocket a tutti i membri online (operazione non bloccante)
    // 9. Ritornare StatusCode::OK

    // L'Owner non può lasciare la chat
    if matches!(metadata.user_role, Some(UserRole::Owner)) {
        warn!("Owner attempted to leave chat");
        return Err(AppError::conflict(
            "The owner cannot leave the chat. Transfer ownership or delete the chat.",
        ));
    }

    state.meta.delete(&(current_user.user_id, chat_id)).await?;

    // Se l'utente è online, inviare segnale RemoveChat per disiscriversi dai messaggi della chat
    state
        .users_online
        .send_server_message_if_online(&current_user.user_id, InternalSignal::RemoveChat(chat_id));

    let message_dto = MessageDTO {
        message_id: None,
        chat_id: Some(chat_id),
        sender_id: Some(current_user.user_id),
        content: Some(format!("User {} has left the chat", current_user.username)),
        message_type: Some(MessageType::SystemMessage),
        created_at: Some(Utc::now()),
    };

    let create_dto = CreateMessageDTO::try_from(message_dto.clone())
    .map_err(|_| AppError::bad_request("Failed to build message dto"))?;

    create_dto
        .validate()
    .map_err(|_| AppError::bad_request("Validation error"))?;

    let _saved_message = state.msg.create(&create_dto).await?;

    let _ = state.chats_online.send(&chat_id, Arc::new(message_dto));
    info!("User successfully left chat");
    Ok(())
}

#[instrument(skip(state, current_user, current_metadata), fields(chat_id = %chat_id, removing_user = %current_user.user_id, target_user = %user_id))]
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    Path((chat_id, user_id)): Path<(i32, i32)>,
    Extension(current_user): Extension<User>,
    Extension(current_metadata): Extension<UserChatMetadata>, // ottenuto dal chat_membership_middleware
) -> Result<(), AppError> {
    debug!("Removing member from chat");
    // 1. Estrarre chat_id e user_id dal path dalla URL
    // 2. Ottenere l'utente corrente e metadata dall'Extension
    // 3. Verificare che current_user sia Admin o Owner, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 4. Recuperare metadata dell'utente target per verificare membership (singola query)
    // 5. Verificare che non si stia cercando di rimuovere l'Owner, altrimenti ritornare errore FORBIDDEN (controllo in memoria)
    // 6. Cancellare i metadata dell'utente target per questa chat dal database
    // 7. Creare un messaggio di sistema che notifica la rimozione del membro (i messaggi dell'utente rimangono nel DB)
    // 8. Salvare il messaggio nel database dopo validazione
    // 9. Inviare il messaggio tramite WebSocket a tutti i membri online della chat (operazione non bloccante)
    // 10. Ritornare StatusCode::OK

    require_role(&current_metadata, &[UserRole::Admin, UserRole::Owner])?;

    let target_meta = state.meta.read(&(user_id, chat_id)).await?.ok_or_else(|| {
        warn!("Target user {} is not a member of chat {}", user_id, chat_id);
        AppError::not_found("The user to be removed is not a member of this chat")
    })?;

    // Non si può rimuovere l'Owner
    if matches!(target_meta.user_role, Some(UserRole::Owner)) {
        warn!("Attempted to remove owner from chat");
        return Err(AppError::forbidden("You cannot remove the owner of the chat"));
    }

    state.meta.delete(&(user_id, chat_id)).await?;

    let target_user_opt = state.user.read(&user_id).await?;

    let target_username = target_user_opt
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "Unknown User".to_string());

    let message_dto = MessageDTO {
        message_id: None,
        chat_id: Some(chat_id),
        sender_id: Some(current_user.user_id),
        content: Some(format!(
            "User {} has removed {} from the chat",
            current_user.username, target_username
        )),
        message_type: Some(MessageType::SystemMessage),
        created_at: Some(Utc::now()),
    };

    let create_dto = CreateMessageDTO::try_from(message_dto.clone())
        .map_err(|_| AppError::bad_request("Failed to build message dto"))?;

    create_dto
        .validate()
        .map_err(|_| AppError::bad_request("Validation error"))?;

    let _saved_message = state.msg.create(&create_dto).await?;

    let _ = state.chats_online.send(&chat_id, Arc::new(message_dto));
    info!("Member successfully removed from chat");
    Ok(())
}

#[debug_handler]
#[instrument(skip(state, current_user, current_metadata, body), fields(chat_id = %chat_id, updating_user = %current_user.user_id, target_user = %user_id, new_role = ?body))]
pub async fn update_member_role(
    State(state): State<Arc<AppState>>,
    Path((chat_id, user_id)): Path<(i32, i32)>,
    Extension(current_user): Extension<User>,
    Extension(current_metadata): Extension<UserChatMetadata>, // ottenuto dal chat_membership_middleware
    Json(body): Json<UserRole>,
) -> Result<(), AppError> {
    debug!("Updating member role in chat");
    // 1. Estrarre user_id e chat_id dal path della URL, nuovo ruolo dal body JSON
    // 2. Ottenere l'utente corrente e metadata dall'Extension
    // 3. Verificare che current_user sia Admin o Owner, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 4. Recuperare metadata dell'utente target per verificare membership (singola query)
    // 5. Verificare le regole di promozione: Owner può modificare tutti, Admin può modificare solo Member (controllo in memoria)
    // 6. Admin non può assegnare ruolo Owner (controllo in memoria)
    // 7. Aggiornare il campo user_role nei metadata dell'utente target
    // 8. Creare un messaggio di sistema che notifica il cambio di ruolo
    // 9. Salvare il messaggio nel database dopo validazione
    // 10. Inviare il messaggio tramite WebSocket a tutti i membri online della chat (operazione non bloccante)
    // 11. Ritornare StatusCode::OK

    require_role(&current_metadata, &[UserRole::Admin, UserRole::Owner])?;

    let target_meta = state.meta.read(&(user_id, chat_id)).await?.ok_or_else(|| {
        warn!("Target user {} is not a member of chat {}", user_id, chat_id);
        AppError::not_found(
            "The user whose role is to be changed is not a member of this chat",
        )
    })?;

    // Nessuno può assegnare il ruolo Owner tramite questo endpoint
    // Per trasferire ownership, usare l'endpoint dedicato transfer_ownership
    if body == UserRole::Owner {
        warn!("Attempted to assign owner role via update_member_role");
        return Err(AppError::forbidden("Cannot assign Owner role. Use transfer_ownership endpoint instead"));
    }

    match current_metadata.user_role {
        Some(UserRole::Admin) => {
            // Admin può modificare solo Member
            match target_meta.user_role {
                Some(UserRole::Member) => { /* ok */ }
                _ => {
                    warn!("Admin attempted to modify non-member role");
                    return Err(AppError::forbidden("Admin can modify only members"));
                }
            }
        }
        _ => { /* current user è Owner: può modificare tutti tranne assegnare Owner (già verificato sopra) */ }
    }

    state
        .meta
        .update_user_role(&user_id, &chat_id, &body)
        .await?;

    let target_user_opt = state.user.read(&user_id).await?;

    let target_username = target_user_opt
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "Unknown User".to_string());

    let message_dto = MessageDTO {
        message_id: None,
        chat_id: Some(chat_id),
        sender_id: Some(current_user.user_id),
        content: Some(format!(
            "User {} has changed {}'s role to {:?}",
            current_user.username, target_username, body
        )),
        message_type: Some(MessageType::SystemMessage),
        created_at: Some(Utc::now()),
    };

    let create_dto = CreateMessageDTO::try_from(message_dto.clone())
    .map_err(|_| AppError::bad_request("Failed to build message dto"))?;

    create_dto
        .validate()
    .map_err(|_| AppError::bad_request("Validation error"))?;

    let _saved_message = state.msg.create(&create_dto).await?;

    let _ = state.chats_online.send(&chat_id, Arc::new(message_dto));
    info!("Member role updated successfully");
    Ok(())
}

#[instrument(skip(state, current_user, metadata), fields(chat_id = %chat_id, current_owner = %current_user.user_id, new_owner = %new_owner_id))]
pub async fn transfer_ownership(
    State(state): State<Arc<AppState>>,
    Path((chat_id, new_owner_id)): Path<(i32, i32)>,
    Extension(current_user): Extension<User>,
    Extension(metadata): Extension<UserChatMetadata>, // ottenuto dal chat_membership_middleware
) -> Result<(), AppError> {
    debug!("Transferring chat ownership");
    // 1. Estrarre chat_id e new_owner_id dal path della URL
    // 2. Ottenere l'utente corrente e metadata dall'Extension
    // 3. Verificare che current_user sia Owner tramite metadata
    // 4. Verificare che current_user non stia trasferendo a se stesso (controllo in memoria)
    // 5. Verificare che la chat esista e sia di tipo Group (le chat private non hanno owner)
    // 6. Verificare che il nuovo owner esista come utente nel sistema
    // 7. Trasferire ownership con metodo atomico: current_user diventa Admin, new_owner diventa Owner
    // 8. Creare un messaggio di sistema che notifica il trasferimento di ownership
    // 9. Salvare il messaggio nel database dopo validazione
    // 10. Inviare il messaggio tramite WebSocket a tutti i membri online della chat (operazione non bloccante)
    // 11. Ritornare StatusCode::OK

    require_role(&metadata, &[UserRole::Owner])?;

    if current_user.user_id == new_owner_id {
        warn!("Attempted to transfer ownership to self");
        return Err(AppError::bad_request(
            "Cannot transfer ownership to yourself",
        ));
    }

    let chat = state.chat.read(&chat_id).await?;

    if let Some(chat_data) = chat {
        if chat_data.chat_type != ChatType::Group {
            warn!("Attempted to transfer ownership of private chat");
            return Err(AppError::bad_request(
                "Cannot transfer ownership of private chats",
            ));
        }
    } else {
        warn!("Chat not found: {}", chat_id);
        return Err(AppError::not_found("Chat not found"));
    }

    let new_owner_user = state.user.read(&new_owner_id).await?;

    let new_owner_username = match new_owner_user {
        Some(user) => user.username,
        None => {
            warn!("New owner user not found: {}", new_owner_id);
            return Err(AppError::not_found("New owner user not found"));
        }
    };

    // Verifica che il nuovo owner sia membro della chat
    let new_owner_meta = state.meta.read(&(new_owner_id, chat_id)).await?;
    if new_owner_meta.is_none() {
        warn!("User {} is not a member of chat {}", new_owner_id, chat_id);
        return Err(AppError::not_found("New owner must be a member of the chat"));
    }

    debug!("Performing ownership transfer");
    // Trasferisce la proprietà dal current_user al nuovo owner
    state
        .meta
        .transfer_ownership(&current_user.user_id, &new_owner_id, &chat_id)
        .await?;

    let message_dto = MessageDTO {
        message_id: None,
        chat_id: Some(chat_id),
        sender_id: Some(current_user.user_id),
        content: Some(format!(
            "User {} has transferred ownership to {}",
            current_user.username, new_owner_username
        )),
        message_type: Some(MessageType::SystemMessage),
        created_at: Some(Utc::now()),
    };

    let create_dto = CreateMessageDTO::try_from(message_dto.clone())
        .map_err(|e| AppError::bad_request("Failed to build message dto").with_details(e.to_string()))?;

    create_dto
        .validate()
        .map_err(|e| AppError::bad_request("Validation error").with_details(e.to_string()))?;

    let _saved_message = state.msg.create(&create_dto).await?;

    let _ = state.chats_online.send(&chat_id, Arc::new(message_dto));
    info!("Ownership transferred successfully");
    Ok(())
}
