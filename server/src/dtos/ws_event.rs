//! WebSocket Event DTOs - Data Transfer Objects per eventi WebSocket

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
    Message(MessageDTO),
    Invitation(InvitationDTO),
    // ... altri eventi
}
