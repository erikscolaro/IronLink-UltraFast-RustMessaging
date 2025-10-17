//! WebSocket Event Handlers - Handler per eventi WebSocket

use crate::core::AppError;
use crate::AppState;
use crate::dtos::{WsEventDTO, CreateMessageDTO, CreateInvitationDTO, InvitationDTO, MessageDTO};
use crate::repositories::{Create, Read};
use crate::entities::UserRole;
use std::sync::Arc;
use tokio::task;

/// Handler per messaggi di chat
/// Operazioni:
/// 1. Validare il messaggio (chat esiste? utente è membro?)
/// 2. Salvare il messaggio nel database
/// 3. Inoltrare il messaggio a tutti i membri online della chat
pub async fn process_chat_message(state: Arc<AppState>, user_id: i32, event: MessageDTO) {
    /*
    - controllare se la chat con l'utente esiste
    - salvare a db il messaggio
     */

    /*
    state.user_online.get(event.destinatario) => tx (se online) altrimenti none
    se online => tx.send(event)
     */
    
    // 1) Verifica che la chat esista
    let chat_id = match event.chat_id {
        Some(id) => id,
        None => {
            AppError::bad_request("Missing chat_id in message event");
            return;
        }
    };

    match state.chat.read(&chat_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            AppError::not_found(format!("Chat {} not found", chat_id));
            return;
        }
        Err(err) => {
            eprintln!("process_chat_message: db error reading chat {}: {:?}", chat_id, err);
            return;
        }
    }

    // 2) Verifica che l'utente sia membro della chat
    match state
        .meta
        .find_by_user_and_chat_id(&user_id, &chat_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            AppError::unauthorized(format!(
                "User {} is not a member of chat {}",
                user_id, chat_id
            ));
            return;
        }
        Err(err) => {
            eprintln!(
                "process_chat_message: db error checking membership for user {} chat {}: {:?}",
                user_id, chat_id, err
            );
            return;
        }
    }

    // 3) Persistere il messaggio nel DB (costruisce CreateMessageDTO da MessageDTO)
    let create_msg = CreateMessageDTO {
        chat_id: chat_id,
        sender_id: user_id,
        // assume `content` è il campo del DTO; se ha nome diverso, adattare qui
        content: event.content.clone().unwrap(),
        message_type: event.message_type.clone().unwrap(),
        created_at: event.created_at.clone().unwrap(),
    };

    let message = match state.msg.create(&create_msg).await {
        Ok(m) => m,
        Err(err) => {
            eprintln!(
                "process_chat_message: failed to create message in chat {}: {:?}",
                chat_id, err
            );
            return;
        }
    };

    // 4) Recupera i membri della chat e inoltra ai membri online (escludendo il mittente)
    let members = match state.meta.find_many_by_chat_id(&chat_id).await {
        Ok(m) => m,
        Err(err) => {
            eprintln!(
                "process_chat_message: failed to load members for chat {}: {:?}",
                chat_id, err
            );
            return;
        }
    };

    for member in members {
        if member.user_id != user_id {
            continue;
        }

        if let Some(tx) = state.users_online.get(&member.user_id) {
            let ev = WsEventDTO::Message(event.clone());
            let tx = tx.clone();
            task::spawn(async move {
                if let Err(send_err) = tx.send(ev).await {
                    eprintln!(
                        "process_chat_message: failed to send WS event to user {}: {:?}",
                        member.user_id, send_err
                    );
                }
            });
        }
    }
}

/// Handler per inviti a chat
/// Operazioni:
/// 1. Validare l'invito (chat esiste? utente ha permessi di invitare?)
/// 2. Salvare l'invito nel database
/// 3. Notificare l'utente invitato se è online
pub async fn process_invitation(state: Arc<AppState>, user_id: i32, event: InvitationDTO) {
    // 1) Estrarre invitee dall'evento
    let invitee_id = event.invitee_id.unwrap(); // assumiamo che sia sempre presente
    let target_chat_id = event.target_chat_id.unwrap(); // assumiamo che sia sempre presente

    // 3) Verificare che l'invitante sia membro (e opzionalmente abbia permessi di invitare)
    let _invitee_data = match state.meta.find_by_user_and_chat_id(&user_id, &target_chat_id).await {
        //ho trovato l'utente nella chat, ha i permessi?
        Ok(Some(data)) => match data.user_role.clone().unwrap() {
            UserRole::Member => {           // Member non ha i permessi di invitare
                AppError::unauthorized(format!(
                    "User {} does not have permission to invite in chat {}",
                    user_id, target_chat_id
                ));
                return;
            },
            _ => data, // Admin o Owner hanno i permessi    
        },
        // l'utente non è membro della chat
        Ok(None) => {
            AppError::unauthorized(format!(
                "User {} is not a member of chat {}",
                user_id, target_chat_id
            ));
            return;
        }
        // errore di accesso al DB
        Err(err) => {
            eprintln!(
                "process_invitation: db error checking membership for user {} chat {}: {:?}",
                user_id, target_chat_id, err
            );
            return;
        }
    };
    
    

    // 4) Creare e salvare l'invito nel DB
    let create_inv = CreateInvitationDTO {
        target_chat_id: target_chat_id,
        invitee_id: user_id,
        invited_id: event.invitee_id.unwrap(),
    };

    if let Err(err) = state.invitation.create(&create_inv).await {
        eprintln!(
            "process_invitation: failed to persist invitation for chat {}: {:?}",
            target_chat_id, err
        );
        return;
    }

    // 5) Notificare l'utente invitato se è online
    if let Some(tx) = state.users_online.get(&invitee_id) {
        let tx = tx.clone();
        let note = WsEventDTO::Invitation(event.clone());
        task::spawn(async move {
            if let Err(send_err) = tx.send(note).await {
                eprintln!(
                    "process_invitation: failed to send invitation notification to user {}: {:?}",
                    invitee_id, send_err
                );
            }
        });
    }
}
