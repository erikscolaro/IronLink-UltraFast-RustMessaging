//! DTOs module - Data Transfer Objects
//!
//! Questo modulo contiene tutti i DTOs usati per la comunicazione client-server.
//! I DTOs separano la rappresentazione esterna (API) dalla rappresentazione interna (entities).

pub mod user;
pub mod chat;
pub mod message;
pub mod invitation;
pub mod user_chat_metadata;
pub mod query;
pub mod ws_event;

// Re-exports per mantenere la compatibilità con il codice esistente
pub use user::{CreateUserDTO, UserDTO};
pub use chat::{ChatDTO, CreateChatDTO};
pub use message::{CreateMessageDTO, MessageDTO};
pub use invitation::{CreateInvitationDTO, InvitationDTO};
pub use user_chat_metadata::{CreateUserChatMetadataDTO, UserInChatDTO};
pub use query::SearchQueryDTO;
pub use ws_event::WsEventDTO;
