//! UserChatMetadata DTOs - Data Transfer Objects per metadati utente-chat

use crate::entities::{UserChatMetadata, UserRole};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Struct per restituire info più semplici per le liste
/// (nome = UserInChatDTO nell'originale, rappresenta un utente in una chat con il suo ruolo)
#[derive(Serialize, Deserialize, Debug)]
pub struct UserInChatDTO {
    pub user_id: Option<i32>,
    pub chat_id: Option<i32>,
    pub username: Option<String>,
    pub user_role: Option<UserRole>,
    pub member_since: Option<DateTime<Utc>>,
    //pub messages_visible_from: Option<DateTime<Utc>>,         // superfluo per il tipo di operazione
    //pub messages_received_until: Option<DateTime<Utc>>,       // superfluo per il tipo di operazione
}

impl From<UserChatMetadata> for UserInChatDTO {
    fn from(value: UserChatMetadata) -> Self {
        Self {
            user_id: Some(value.user_id),
            chat_id: Some(value.chat_id),
            username: None, // Non è presente in UserChatMetadata, va popolato altrove
            user_role: value.user_role,
            member_since: Some(value.member_since),
            // messages_visible_from: Some(value.messages_visible_from),
            // messages_received_until: Some(value.messages_received_until),
        }
    }
}

/// DTO per creare nuovi metadati utente-chat (senza member_since, messages_visible_from, messages_received_until - gestiti dal DB)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateUserChatMetadataDTO {
    pub user_id: i32,
    pub chat_id: i32,
    pub user_role: Option<UserRole>,
    pub member_since: DateTime<Utc>,
    pub messages_visible_from: DateTime<Utc>,
    pub messages_received_until: DateTime<Utc>,
}

/// DTO per aggiornare metadati utente-chat (solo campi modificabili)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateUserChatMetadataDTO {
    pub user_role: Option<UserRole>,
    pub messages_visible_from: Option<DateTime<Utc>>,
    pub messages_received_until: Option<DateTime<Utc>>,
}
