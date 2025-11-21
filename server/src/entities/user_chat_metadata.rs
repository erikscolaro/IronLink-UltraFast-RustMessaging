//! UserChatMetadata entity - Entità metadata utente-chat

use super::enums::UserRole;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserChatMetadata {
    pub user_id: i32,
    pub chat_id: i32,
    pub user_role: Option<UserRole>,
    pub member_since: DateTime<Utc>,
    // sostituisce deliver_from con un nome più esplicativo
    // sostituito al posto dell'id del messaggio il datetime, è da intendersi come
    // "visualizza i messaggi da questo istante in poi, questo istante ESCLUSO"
    pub messages_visible_from: DateTime<Utc>,
    // sostituisce last delivered con un nume più esplicativo
    // sostituito al posto dell'id del messaggio il date time, è da intendersi come
    // "ho ricevuto i messaggi fino a questo istante, istante INCLUSO"
    pub messages_received_until: DateTime<Utc>,
    //per ora non esludo i due campi dalla deserializzazione
}
