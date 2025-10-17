//! Enumerazioni - Tipi enumerati utilizzati nelle entità

use serde::{Deserialize, Serialize};

// ********************* ENUMERAZIONI UTILI **********************//

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, sqlx::Type)]
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
    Member,
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "invitation_status", rename_all = "UPPERCASE")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::Type, PartialEq)]
#[sqlx(type_name = "chat_type", rename_all = "UPPERCASE")]
pub enum ChatType {
    Group,
    Private,
}
