use dashmap::DashMap;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{info, instrument, warn};

use crate::dtos::{InvitationDTO, user};

pub enum InternalSignal {
    Shutdown,
    AddChat(i32),
    RemoveChat(i32),
    Error(&'static str),
    Invitation(InvitationDTO),
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
        info!("Registering user as online");
        self.users_online.insert(user_id, tx);
    }

    #[instrument(skip(self), fields(user_id))]
    pub fn remove_from_online(&self, user_id: &i32) {
        info!("Removing user from online");
        self.users_online.remove(&user_id);
    }

    #[instrument(skip(self, message), fields(user_id))]
    pub fn send_server_message_if_online(&self, user_id: &i32, message: InternalSignal) {
        if let Some(entry) = self.users_online.get(&user_id) {
            let tx = entry.value();
            if let Err(e) = tx.send(message) {
                warn!("Failed to send message to user: {:?}", e);
            } else {
                info!("Message sent to online user");
            }
        } else {
            info!("User not online, message not sent");
        }
    }
}
