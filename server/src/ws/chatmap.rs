use crate::dtos::WsEventDTO;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::{Receiver, Sender};
use crate::ws::BROADCAST_CHANNEL_CAPACITY;

pub struct ChatMap {
    /// Attribute to retrieve the tx head og a broadcast channel by chat_id field
    channels: DashMap<i32, Sender<Arc<WsEventDTO>>>,
}

impl ChatMap {
    pub fn new() -> Self {
        ChatMap {
            channels: DashMap::new(),
        }
    }

    pub fn subscribe(&self, chat_id: &i32) -> Receiver<Arc<WsEventDTO>> {
        match self.channels.get(chat_id) {
            // required subscription on non existing chat channel
            None => {
                // Arc<Message> to share the ref, not the message. Avoid unuseful copies of message on each rx.
                let (tx, rx) = broadcast::channel::<Arc<WsEventDTO>>(BROADCAST_CHANNEL_CAPACITY);
                self.channels.insert(*chat_id, tx);
                rx
            }
            // subscribe to an existing channel == get a rx head == subscribe to a tx
            Some(c) => c.value().subscribe(),
        }
    }

    pub fn subscribe_multiple(&self, chat_ids: Vec<i32>) -> Vec<Receiver<Arc<WsEventDTO>>> {
        chat_ids.into_iter().map(|id| self.subscribe(&id)).collect()
    }

    pub fn send(&self, chat_id: &i32, msg: Arc<WsEventDTO>) -> Result<usize, SendError<Arc<WsEventDTO>>>{
        if let Some(chat) = self.channels.get(chat_id){
            match chat.send(msg.clone()) {
                Ok(n) => Ok(n),
                Err(e) => {
                    // Nessuno sta ascoltando, rimuovi il channel
                    drop(chat); // Rilascia il lock
                    self.channels.remove(chat_id);
                    Err(e)
                }
            }
        } else {
            Err(SendError(msg))
        }
    }
}
