//! Message DTOs - Data Transfer Objects per messaggi

use crate::entities::{Message, MessageType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Struct per gestire io col client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageDTO {
    pub message_id: Option<i32>,
    pub chat_id: Option<i32>,
    pub sender_id: Option<i32>,
    pub content: Option<String>,
    pub message_type: Option<MessageType>,
    pub created_at: Option<DateTime<Utc>>,
}

impl From<Message> for MessageDTO {
    fn from(value: Message) -> Self {
        Self {
            message_id: Some(value.message_id),
            chat_id: Some(value.chat_id),
            sender_id: Some(value.sender_id),
            content: Some(value.content),
            message_type: Some(value.message_type),
            created_at: Some(value.created_at),
        }
    }
}

/// DTO per creare un nuovo messaggio (senza message_id)
#[derive(Serialize, Deserialize, Debug, Clone, Validate)]
pub struct CreateMessageDTO {
    pub chat_id: i32,
    pub sender_id: i32,
    
    #[validate(length(min = 1, max = 5000, message = "Message content must be between 1 and 5000 characters"))]
    pub content: String,
    
    pub message_type: MessageType,
    pub created_at: DateTime<Utc>,
}

/// DTO per aggiornare un messaggio (solo campi modificabili)
#[derive(Serialize, Deserialize, Debug, Clone, Validate)]
pub struct UpdateMessageDTO {
    #[validate(length(min = 1, max = 5000, message = "Message content must be between 1 and 5000 characters"))]
    pub content: Option<String>,
}
