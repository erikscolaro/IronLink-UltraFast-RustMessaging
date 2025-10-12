//! Entities module - Entità del dominio applicativo
//!
//! Questo modulo contiene tutte le entità (models) che rappresentano i dati persistiti nel database.
//! Ogni entity corrisponde a una tabella nel database.

pub mod enums;
pub mod user;
pub mod message;
pub mod chat;
pub mod user_chat_metadata;
pub mod invitation;

// Re-exports per facilitare l'import
pub use enums::{ChatType, InvitationStatus, MessageType, UserRole};
pub use user::User;
pub use message::Message;
pub use chat::Chat;
pub use user_chat_metadata::UserChatMetadata;
pub use invitation::Invitation;
