/*
pacchetto di idee:
- singleton per distribuire le connessioni a db tramite pool in modo da evitre
 di passare ad ogni entitò una connessione
- pacchetto tratto crud
- singleton per la struttura hashmap utente - websocket
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ********************* ENUMERAZIONI UTILI **********************//
#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    UserMessage,
    SystemMessage,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum UserRole {
    Owner,
    Admin,
    Standard,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ChatType {
    Group,
    Private,
}

// definisco un alias di tipo per id, in modo che se vogliamo switchare a uuid lo possiamo fare veloci
pub type IdType = u32;

// ********************* MODELLI VERI E PROPRI *******************//

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    /* se vogliamo rinominare campi usiamo la macro
     * #[serde(rename = "userId")]
     */
    id: IdType,
    username: String,
    // Questo per evitare la serializzazione quando inviamo le informazioni utente al client
    #[serde(skip_deserializing)]
    password_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    id: IdType,
    chat_id: IdType,
    sender_id: IdType,
    content: String,
    // il server si aspetta una stringa litterale iso8601 che viene parsata in oggetto DateTime di tipo UTC
    // la conversione viene fatta in automatico da serde, la feature è stata abilitata
    created_at: DateTime<Utc>,
    // campo rinominato rispetto a uml perchè type è una parola protetta
    message_type: MessageType,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UserChatMetadata {
    user_id: IdType,
    chat_id: IdType,
    user_role: UserRole,
    // sostituisce deliver_from con un nome più esplicativo
    // sostituito al posto dell'id del messaggio il datetime, è da intendersi come
    // "visualizza i messaggi da questo istante in poi, questo istante ESCLUSO"
    messages_visible_from: DateTime<Utc>,
    // sostituisce last delivered con un nume più esplicativo
    // sostituito al posto dell'id del messaggio il date time, è da intendersi come
    // "ho ricevuto i messaggi fino a questo istante, istante INCLUSO"
    messages_received_until: DateTime<Utc>,
    //per ora non esludo i due campi dalla deserializzazione
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Invitation {
    id: IdType,
    // rinominato da group_id a chat_id per consistenza
    chat_id: IdType,    // chat ( di gruppo ) in cui si viene invitati
    invited_id: IdType, // utente invitato
    invitee_id: IdType, // utente che invita
    state: InvitationStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Chat {
    id: IdType,
    title: Option<String>,
    description: Option<String>,
    chat_type: ChatType,
}
