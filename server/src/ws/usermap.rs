use dashmap::DashMap;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, instrument, warn};

use crate::dtos::EnrichedInvitationDTO;

pub enum InternalSignal {
    Shutdown,
    AddChat(i32),
    RemoveChat(i32),
    Error(&'static str),
    Invitation(EnrichedInvitationDTO),
}

pub struct UserMap {
    users_online: DashMap<i32, UnboundedSender<InternalSignal>>,
}

impl UserMap {
    pub fn new() -> Self {
        UserMap {
            users_online: DashMap::new(),
        }
    }

    #[instrument(skip(self, tx), fields(user_id))]
    pub fn register_online(&self, user_id: i32, tx: UnboundedSender<InternalSignal>) {
        info!("Registering user {} as online", user_id);
        self.users_online.insert(user_id, tx);
        info!("Total online users: {}", self.users_online.len());
    }

    #[instrument(skip(self), fields(user_id))]
    pub fn remove_from_online(&self, user_id: &i32) {
        info!("Removing user from online");
        self.users_online.remove(&user_id);
    }

    #[instrument(skip(self, message), fields(user_id))]
    pub fn send_server_message_if_online(&self, user_id: &i32, message: InternalSignal) {
        let message_type = match &message {
            InternalSignal::Shutdown => "Shutdown",
            InternalSignal::AddChat(chat_id) => {
                info!("Sending AddChat signal for chat_id {}", chat_id);
                "AddChat"
            }
            InternalSignal::RemoveChat(chat_id) => {
                info!("Sending RemoveChat signal for chat_id {}", chat_id);
                "RemoveChat"
            }
            InternalSignal::Error(_) => "Error",
            InternalSignal::Invitation(inv) => {
                info!("Sending Invitation signal for invite_id {}", inv.invite_id);
                "Invitation"
            }
        };

        if let Some(entry) = self.users_online.get(&user_id) {
            let tx = entry.value();
            if let Err(e) = tx.send(message) {
                warn!("Failed to send {} message to user: {:?}", message_type, e);
            } else {
                info!("{} message sent to online user", message_type);
            }
        } else {
            info!("User {} not online, {} message not sent", user_id, message_type);
        }
    }

    /// Get the count of online users
    #[allow(dead_code)]
    pub fn online_count(&self) -> usize {
        self.users_online.len()
    }

    /// Check if a specific user is online
    #[allow(dead_code)]
    pub fn is_user_online(&self, user_id: &i32) -> bool {
        self.users_online.contains_key(user_id)
    }
}

