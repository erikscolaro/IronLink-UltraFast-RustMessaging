//! WebSocket Event DTOs - Data Transfer Objects per eventi WebSocket

use crate::entities::{Message, User};
use serde::{Deserialize, Serialize};

use crate::dtos::{InvitationDTO, MessageDTO};

/// Enum per gestire gli eventi WebSocket in modo type-safe
/// Tagged union per eventi WebSocket
/// Serde serializza questo come:
/// { "type": "NewMessage", "data": { ... } }
/// oppure
/// { "type": "UserJoined", "data": { ... } }
/// etc.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum WsEventDTO {
    NewMessage(Message),
    UserJoined(User),
    UserLeft(User),
    Message(MessageDTO),
    Invitation(InvitationDTO),
    Error { code: u16, message: String },
    // ... altri eventi
}
