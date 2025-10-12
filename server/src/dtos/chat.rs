//! Chat DTOs - Data Transfer Objects per chat

use crate::entities::{Chat, ChatType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Struct per gestire io col client
#[derive(Serialize, Deserialize, Debug)]
pub struct ChatDTO {
    pub chat_id: Option<i32>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub chat_type: Option<ChatType>,
    pub user_list: Option<Vec<i32>>, // lista user_id per chat private/gruppo
}

impl From<Chat> for ChatDTO {
    fn from(value: Chat) -> Self {
        Self {
            chat_id: Some(value.chat_id),
            title: value.title,
            description: value.description,
            chat_type: Some(value.chat_type),
            user_list: None, // da popolare manualmente se necessario
        }
    }
}

/// DTO per creare una nuova chat (senza chat_id)
#[derive(Serialize, Deserialize, Debug, Clone, Validate)]
pub struct CreateChatDTO {
    #[validate(length(min = 1, max = 100, message = "Chat title must be between 1 and 100 characters"))]
    pub title: Option<String>,
    
    #[validate(length(max = 500, message = "Chat description must not exceed 500 characters"))]
    pub description: Option<String>,
    
    pub chat_type: ChatType,
}

/// DTO per aggiornare una chat (solo campi modificabili)
#[derive(Serialize, Deserialize, Debug, Clone, Validate)]
pub struct UpdateChatDTO {
    #[validate(length(min = 1, max = 100, message = "Chat title must be between 1 and 100 characters"))]
    pub title: Option<String>,
    
    #[validate(length(max = 500, message = "Chat description must not exceed 500 characters"))]
    pub description: Option<String>,
}
