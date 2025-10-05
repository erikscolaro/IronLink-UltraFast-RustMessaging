use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ********************* ENUMERAZIONI UTILI **********************//
#[derive(Serialize, Deserialize, Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "message_type", rename_all = "UPPERCASE")]
pub enum MessageType {
    UserMessage,
    SystemMessage,
}
#[derive(Serialize, Deserialize, Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "UPPERCASE")]
pub enum UserRole {
    Owner,
    Admin,
    Standard,
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "invitation_status", rename_all = "UPPERCASE")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "chat_type", rename_all = "UPPERCASE")]
#[derive(PartialEq)]
pub enum ChatType {
    Group,
    Private,
}

// ********************* MODELLI VERI E PROPRI *******************//

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    /* se vogliamo rinominare campi usiamo la macro
     * #[serde(rename = "userId")]
     */
    pub user_id: i32,
    pub username: String,
    pub password: String,
}

impl User {
    // Verify if target_password matches the stored hashed password
    pub fn verify_password(&self, target_password: &String) -> bool {
        verify(target_password, &self.password).unwrap_or_else(|_| false)
    }

    pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
        let hash = hash(password, DEFAULT_COST)?;
        Ok(hash)
    }
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Invitation {
    pub invite_id: i32, //todo: valutare se togliere la chaive primaria qui, gli inviti sono univoci per invitee e chat_id
    // rinominato da group_id a chat_id per consistenza
    pub target_chat_id: i32, // chat ( di gruppo ) in cui si viene invitati
    pub invited_id: i32,     // utente invitato
    pub invitee_id: i32,     // utente che invita
    pub state: InvitationStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chat {
    pub chat_id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub chat_type: ChatType,
}
