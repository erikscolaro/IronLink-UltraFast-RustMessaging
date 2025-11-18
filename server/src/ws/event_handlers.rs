//! WebSocket Event Handlers - Handler per eventi WebSocket

use tracing::{error, info, instrument, warn};
use validator::Validate;

use crate::AppState;
use crate::dtos::{CreateMessageDTO, MessageDTO};
use crate::entities::MessageType;
use crate::repositories::{Create, Read};
use crate::ws::usermap::InternalSignal;
use std::sync::Arc;

#[instrument(skip(state, msg), fields(user_id, chat_id = msg.chat_id))]
pub async fn process_message(state: &Arc<AppState>, user_id: i32, msg: MessageDTO) {
    info!("Processing message from user");

    let input_message = match CreateMessageDTO::try_from(msg.clone()) {
        Ok(msg) => msg,
        Err(e) => {
            warn!("Malformed message received: {:?}", e);
            state.users_online.send_server_message_if_online(
                &user_id,
                InternalSignal::Error("Malformed message."),
            );
            return;
        }
    };

    if let Err(e) = input_message.validate() {
        warn!("Message validation failed: {:?}", e);
        state
            .users_online
            .send_server_message_if_online(&user_id, InternalSignal::Error("Malformed message."));
        return;
    };

    if input_message.message_type == MessageType::SystemMessage {
        warn!("User attempted to send system message");
        state.users_online.send_server_message_if_online(
            &user_id,
            InternalSignal::Error("You cannot send system type messages."),
        );
        return;
    }

    // Verifica che il sender_id corrisponda all'utente autenticato
    if input_message.sender_id != user_id {
        warn!(
            expected_sender_id = user_id,
            actual_sender_id = input_message.sender_id,
            "User attempted to spoof sender_id"
        );
        state.users_online.send_server_message_if_online(
            &user_id,
            InternalSignal::Error("Malformed message."),
        );
        return;
    }

    // se la chat non esistesse, allora non esisterebbe neanche il metadata, quindi non controllo l'esistenza della chat.
    match state.meta.read(&(user_id, input_message.chat_id)).await {
        Ok(Some(val)) => val,
        Ok(None) => {
            warn!(
                chat_id = input_message.chat_id,
                "User does not belong to chat"
            );
            state.users_online.send_server_message_if_online(
                &user_id,
                InternalSignal::Error("You don't belong to that group."),
            );
            return;
        }
        Err(e) => {
            error!("Failed to read user metadata: {:?}", e);
            state.users_online.send_server_message_if_online(
                &user_id,
                InternalSignal::Error("Internal server error."),
            );
            return;
        }
    };

    // bene, l'utente appartiene alla chat, quindi può inviare il messaggio
    // invio prima ad utenti online (sia per chat private che di gruppo)
    match state
        .chats_online
        .send(&input_message.chat_id, Arc::from(msg))
    {
        Ok(n) => {
            info!(
                chat_id = input_message.chat_id,
                receivers = n,
                "Message broadcast to {} receivers", n
            );
        }
        Err(_) => {
            // Nessun ricevitore online per questa chat (canale non esiste o nessuno iscritto)
            // Questo è normale per chat nuove o quando tutti gli utenti sono offline
            warn!(
                chat_id = input_message.chat_id,
                "No online receivers for this chat, message will be stored for later delivery"
            );
        }
    }

    // salvo in db per utenti offline
    if let Err(e) = state.msg.create(&input_message).await {
        error!("Failed to persist message to database: {:?}", e);
        state.users_online.send_server_message_if_online(
            &user_id,
            InternalSignal::Error(
                "Something went wrong and your message was not stored correctly!",
            ),
        );
    } else {
        info!("Message processed and stored successfully");
    }
}
