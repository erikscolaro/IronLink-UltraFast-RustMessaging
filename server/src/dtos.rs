use crate::entities::{Chat, ChatType, IdType, Message, MessageType, User, UserChatMetadata, UserRole};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
			id: Some(value.id),
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
			id: Some(value.id),
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
	pub member_since: DateTime<Utc>
}

impl From<(User, UserChatMetadata)> for UserInChatDTO {
	fn from(value: (User, UserChatMetadata)) -> Self {
		let (user, meta) = value;
		Self {
			user: UserDTO::from(user),
			role: meta.user_role,
			member_since: meta.member_since
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageDTO {
	pub message_id: IdType,
	pub chat_id: IdType,
	pub sender_id: IdType,
	pub content: String,
	pub created_at: DateTime<Utc>,
	pub message_type: MessageType,
}

impl From<Message> for MessageDTO {
	fn from(msg: Message) -> Self {
		Self {
			message_id: msg.message_id,
			chat_id: msg.chat_id,
			sender_id: msg.sender_id,
			content: msg.content,
			created_at: msg.created_at,
			message_type: msg.message_type,
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchQueryDTO {
	pub search: Option<String>
}




