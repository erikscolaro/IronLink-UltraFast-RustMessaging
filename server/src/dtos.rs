use crate::entities::{Chat, ChatType, IdType, Invitation, InvitationStatus, Message, MessageType, User, UserChatMetadata, UserRole};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::error_handler::AppError;

// struct per gestire io col client
#[derive(Serialize, Deserialize, Debug)]
pub struct UserDTO {
    pub id: Option<IdType>,
    pub username: Option<String>,
    #[serde(skip_serializing)]
    pub password: Option<String>,
}

impl From<User> for UserDTO {
    fn from(value: User) -> Self {
        Self {
            id: Some(value.user_id),
            username: Some(value.username),
            password: None, // mai esposta al client!!!
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatDTO {
    id: Option<IdType>,
    title: Option<String>,
    description: Option<String>,
    chat_type: ChatType,
}

impl From<Chat> for ChatDTO {
    fn from(value: Chat) -> Self {
        Self {
            id: Some(value.chat_id),
            title: value.title,
            description: value.description,
            chat_type: value.chat_type,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInChatDTO {
    pub user: UserDTO,
    pub role: UserRole,
    pub member_since: DateTime<Utc>,
}

impl From<(User, UserChatMetadata)> for UserInChatDTO {
    fn from(value: (User, UserChatMetadata)) -> Self {
        let (user, meta) = value;
        Self {
            user: UserDTO::from(user),
            role: meta.user_role,
            member_since: meta.member_since,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageDTO {
    pub message_id: Option<IdType>, // reso opzionale
    pub chat_id: IdType,
    pub sender_id: IdType,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub message_type: MessageType,
}

impl From<Message> for MessageDTO {
    fn from(msg: Message) -> Self {
        Self {
            message_id: Some(msg.message_id), // incapsulato in Some()
            chat_id: msg.chat_id,
            sender_id: msg.sender_id,
            content: msg.content,
            created_at: msg.created_at,
            message_type: msg.message_type,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InvitationDTO {
    pub invite_id: Option<IdType>, // opzionale
    pub target_chat_id: IdType,
    pub invited_id: IdType,
    pub invitee_id: IdType,
    pub state: InvitationStatus,
}

impl From<Invitation> for InvitationDTO {
    fn from(inv: Invitation) -> Self {
        Self {
            invite_id: Some(inv.invite_id), // incapsulato in Some()
            target_chat_id: inv.target_chat_id,
            invited_id: inv.invited_id,
            invitee_id: inv.invitee_id,
            state: inv.state,
        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct SearchQueryDTO {
    pub search: Option<String>,
}

/*
{
  "type": "Message",
  "data": {
    "message_id": "number",
    "chat_id": "number",
    "sender_id": "number",
    "content": "string",
    "created_at": "ISO8601 timestamp",
    "message_type": "Private|Group|System"
  }
}
{
  "type": "Invitation",
  "data": {
    "invite_id": "number",
    "target_chat_id": "number",
    "invited_id": "number",
    "invitee_id": "number",
    "state": "Pending|Accepted|Rejected"
  }
}
{
  "type": "System",
  "data": {
    "content": "string",
    "created_at": "ISO8601 timestamp"
  }
}

 */

// enumerazione per gestire i vari casi di eventi ws 
// e la deserializzazione
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum WsEventDTO {
    Message(MessageDTO),
    Invitation(InvitationDTO),
    System {
        content: String,
        created_at: DateTime<Utc>,
    },
    Error {
        code: u16,
        message: String,
    },
}
