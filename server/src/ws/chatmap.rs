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

    /// Check if a chat channel exists
    #[allow(dead_code)]
    pub fn has_chat_channel(&self, chat_id: &i32) -> bool {
        self.channels.contains_key(chat_id)
    }
}

/*
Here we need to test chatmap.
We have 4 principal methods
- new
    - a simple test creation, nothing strange
- subscribe
    - branch, canale del gruppo già creato in mappa?
        - si -> subscribe -> verifica subscription, se rx <end>
        - no -> crea il canale in map -> verifica se il canale esiste <end>
- subscribe multiple
    - only a test to verify that functions
- send
    - branch, esiste la chat a cui voglio inviare il messaggio?
        - no -> return error
        - si -> prova a inviare il messaggio, branch:
            - riuscito -> controlla se ritorna OK
            - non riuscito -> dovrebbe togliere il canale dalla mappa e ritornare errore

*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::MessageType;
    use chrono::Utc;

    // Helper function to create a test message
    fn create_test_message(chat_id: i32, content: &str) -> Arc<MessageDTO> {
        Arc::new(MessageDTO {
            message_id: Some(1),
            chat_id: Some(chat_id),
            sender_id: Some(1),
            content: Some(content.to_string()),
            created_at: Some(Utc::now()),
            message_type: Some(MessageType::UserMessage),
        })
    }

    #[test]
    fn test_new_chatmap_creation() {
        // Test 1: Simple creation test
        let chatmap = ChatMap::new();
        assert_eq!(chatmap.channels.len(), 0, "New ChatMap should be empty");
    }

    #[test]
    fn test_subscribe_creates_new_channel() {
        // Test 2: Subscribe to non-existing chat - should create new channel
        let chatmap = ChatMap::new();
        let chat_id = 1;

        let _rx = chatmap.subscribe(&chat_id);

        // Verify the channel was created
        assert!(
            chatmap.channels.contains_key(&chat_id),
            "Channel should be created for new chat_id"
        );
        assert_eq!(
            chatmap.channels.len(),
            1,
            "ChatMap should contain exactly one channel"
        );
    }

    #[test]
    fn test_subscribe_to_existing_channel() {
        // Test 3: Subscribe to existing chat - should return new receiver
        let chatmap = ChatMap::new();
        let chat_id = 1;

        // First subscription creates the channel
        let rx1 = chatmap.subscribe(&chat_id);

        // Second subscription should reuse the existing channel
        let rx2 = chatmap.subscribe(&chat_id);

        // Verify still only one channel exists
        assert_eq!(
            chatmap.channels.len(),
            1,
            "Should still have only one channel"
        );

        // Verify both receivers work by sending a message
        let msg = create_test_message(chat_id, "test message");
        let result = chatmap.send(&chat_id, msg.clone());
        assert!(result.is_ok(), "Send should succeed");
        assert_eq!(result.unwrap(), 2, "Should have 2 receivers");

        // Both receivers should be able to receive
        drop(rx1);
        drop(rx2);
    }

    #[test]
    fn test_subscribe_multiple_chats() {
        // Test 4: Subscribe to multiple chats at once
        let chatmap = ChatMap::new();
        let chat_ids = vec![1, 2, 3, 4, 5];

        let receivers = chatmap.subscribe_multiple(chat_ids.clone());

        // Verify correct number of receivers
        assert_eq!(
            receivers.len(),
            5,
            "Should receive 5 receivers for 5 chat_ids"
        );

        // Verify all channels were created
        assert_eq!(chatmap.channels.len(), 5, "Should have 5 channels created");

        for chat_id in chat_ids {
            assert!(
                chatmap.channels.contains_key(&chat_id),
                "Channel for chat_id {} should exist",
                chat_id
            );
        }
    }

    #[test]
    fn test_send_to_non_existent_chat() {
        // Test 5: Send to non-existent chat - should return error
        let chatmap = ChatMap::new();
        let chat_id = 999;
        let msg = create_test_message(chat_id, "test message");

        let result = chatmap.send(&chat_id, msg.clone());

        assert!(result.is_err(), "Send to non-existent chat should fail");
        assert_eq!(
            chatmap.channels.len(),
            0,
            "No channels should be created on failed send"
        );
    }

    #[test]
    fn test_send_successful_with_receivers() {
        // Test 6: Send to existing chat with active receivers - should succeed
        let chatmap = ChatMap::new();
        let chat_id = 1;

        // Create some receivers
        let mut rx1 = chatmap.subscribe(&chat_id);
        let mut rx2 = chatmap.subscribe(&chat_id);

        let msg = create_test_message(chat_id, "hello world");
        let result = chatmap.send(&chat_id, msg.clone());

        assert!(result.is_ok(), "Send should succeed with active receivers");
        assert_eq!(
            result.unwrap(),
            2,
            "Should have sent to 2 active receivers"
        );

        // Verify receivers got the message
        let received1 = rx1.try_recv();
        let received2 = rx2.try_recv();

        assert!(received1.is_ok(), "Receiver 1 should receive the message");
        assert!(received2.is_ok(), "Receiver 2 should receive the message");
        assert_eq!(
            received1.unwrap().content,
            Some("hello world".to_string()),
            "Message content should match"
        );
        assert_eq!(
            received2.unwrap().content,
            Some("hello world".to_string()),
            "Message content should match"
        );
    }

    #[test]
    fn test_send_removes_channel_when_no_receivers() {
        // Test 7: Send when no active receivers - should remove channel and return error
        let chatmap = ChatMap::new();
        let chat_id = 1;

        // Subscribe and immediately drop the receiver
        {
            let _rx = chatmap.subscribe(&chat_id);
        } // rx is dropped here, no active receivers

        // Verify channel was created
        assert_eq!(chatmap.channels.len(), 1, "Channel should exist initially");

        let msg = create_test_message(chat_id, "test message");
        let result = chatmap.send(&chat_id, msg.clone());

        // Send should fail because no receivers
        assert!(
            result.is_err(),
            "Send should fail when no active receivers"
        );

        // Verify channel was removed from the map
        assert_eq!(
            chatmap.channels.len(),
            0,
            "Channel should be removed when no receivers"
        );
        assert!(
            !chatmap.channels.contains_key(&chat_id),
            "Chat channel should not exist after removal"
        );
    }
}