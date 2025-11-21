//! Entities module - Entità del dominio applicativo
//!
//! Questo modulo contiene tutte le entità (models) che rappresentano i dati persistiti nel database.
//! Ogni entity corrisponde a una tabella nel database.

pub mod chat;
pub mod enums;
pub mod invitation;
pub mod message;
pub mod user;
pub mod user_chat_metadata;

// Re-exports per facilitare l'import
pub use chat::Chat;
pub use enums::{ChatType, InvitationStatus, MessageType, UserRole};
pub use invitation::Invitation;
pub use message::Message;
pub use user::User;
pub use user_chat_metadata::UserChatMetadata;
