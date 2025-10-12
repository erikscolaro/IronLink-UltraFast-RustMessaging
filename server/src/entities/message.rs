//! Message entity - Entità messaggio

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::enums::MessageType;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub message_id: i32,
    pub chat_id: i32,
    pub sender_id: i32, // rendere opzionale per i messaggi di sistema visto che il sistema non ha tipo ?
    pub content: String,
    // il server si aspetta una stringa litterale iso8601 che viene parsata in oggetto DateTime di tipo UTC
    // la conversione viene fatta in automatico da serde, la feature è stata abilitata
    pub created_at: DateTime<Utc>,
    // campo rinominato rispetto a uml perchè type è una parola protetta
    pub message_type: MessageType,
}
