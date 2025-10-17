//! Services module - Coordinatore per tutti i service handler HTTP
//!
//! Questo modulo organizza i service handlers in sotto-moduli separati per una migliore manutenibilità.
//! Ogni modulo gestisce gli endpoint HTTP per una specifica funzionalità.

pub mod auth;
pub mod chat;
pub mod membership;
pub mod user;

// Re-exports per facilitare l'import
pub use auth::{login_user, register_user};
pub use chat::{create_chat, get_chat_messages, list_chats};
pub use membership::{
    invite_to_chat, leave_chat, list_chat_members, list_pending_invitations, remove_member,
    respond_to_invitation, transfer_ownership, update_member_role,
};
pub use user::{delete_my_account, get_user_by_id, search_user_with_username};

use crate::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

/// Root endpoint - health check
pub async fn root(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    (StatusCode::OK, "Server is running!")
}
