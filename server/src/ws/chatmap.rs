use crate::dtos::MessageDTO;
use crate::ws::BROADCAST_CHANNEL_CAPACITY;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::{info, instrument, warn};

pub struct ChatMap {
    /// Attribute to retrieve the tx head og a broadcast channel by chat_id field
    channels: DashMap<i32, Sender<Arc<MessageDTO>>>,
}

impl ChatMap {
    pub fn new() -> Self {
        ChatMap {
            channels: DashMap::new(),
        }
    }

    #[instrument(skip(self), fields(chat_id))]
    pub fn subscribe(&self, chat_id: &i32) -> Receiver<Arc<MessageDTO>> {
        match self.channels.get(chat_id) {
            // required subscription on non existing chat channel
            None => {
                info!("Creating new broadcast channel for chat");
                // Arc<Message> to share the ref, not the message. Avoid unuseful copies of message on each rx.
                let (tx, rx) = broadcast::channel::<Arc<MessageDTO>>(BROADCAST_CHANNEL_CAPACITY);
                self.channels.insert(*chat_id, tx);
                rx
            }
            // subscribe to an existing channel == get a rx head == subscribe to a tx
            Some(c) => {
                info!("Subscribing to existing broadcast channel");
                c.value().subscribe()
            }
        }
    }

    #[instrument(skip(self, chat_ids))]
    pub fn subscribe_multiple(&self, chat_ids: Vec<i32>) -> Vec<Receiver<Arc<MessageDTO>>> {
        info!(count = chat_ids.len(), "Subscribing to multiple chats");
        chat_ids.into_iter().map(|id| self.subscribe(&id)).collect()
    }

    #[instrument(skip(self, msg), fields(chat_id))]
    pub fn send(
        &self,
        chat_id: &i32,
        msg: Arc<MessageDTO>,
    ) -> Result<usize, SendError<Arc<MessageDTO>>> {
        if let Some(chat) = self.channels.get(chat_id) {
            match chat.send(msg.clone()) {
                Ok(n) => {
                    info!(receivers = n, "Message broadcast to receivers");
                    Ok(n)
                }
                Err(e) => {
                    warn!("No active receivers, removing channel");
                    // Nessuno sta ascoltando, rimuovi il channel
                    drop(chat); // Rilascia il lock
                    self.channels.remove(chat_id);
                    Err(e)
                }
            }
        } else {
            warn!("Attempted to send to non-existent chat channel");
            Err(SendError(msg))
        }
    }
}
