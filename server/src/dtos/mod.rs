//! DTOs module - Data Transfer Objects
//!
//! Questo modulo contiene tutti i DTOs usati per la comunicazione client-server.
//! I DTOs separano la rappresentazione esterna (API) dalla rappresentazione interna (entities).

pub mod chat;
pub mod invitation;
pub mod message;
pub mod query;
pub mod user;
pub mod user_chat_metadata;

// Re-exports per mantenere la compatibilità con il codice esistente
pub use chat::{ChatDTO, CreateChatDTO, UpdateChatDTO};
pub use invitation::{CreateInvitationDTO, EnrichedInvitationDTO, UpdateInvitationDTO};
pub use message::{CreateMessageDTO, MessageDTO, UpdateMessageDTO};
pub use query::{MessagesQuery, UserSearchQuery};
pub use user::{CreateUserDTO, UpdateUserDTO, UserDTO};
pub use user_chat_metadata::{CreateUserChatMetadataDTO, UpdateUserChatMetadataDTO, UserInChatDTO};
