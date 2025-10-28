//! Server library - espone i moduli principali per i test

pub mod core;
pub mod dtos;
pub mod entities;
pub mod repositories;
pub mod services;
pub mod ws;

// Re-export dei tipi principali per facilitare l'import
pub use core::{AppState, AppError, auth, config};
pub use services::root;

use axum::{Router, middleware, routing::{any, delete, get, patch, post}};
use std::sync::Arc;

/// Crea il router principale dell'applicazione
pub fn create_router(state: Arc<AppState>) -> Router {
    use core::authentication_middleware;
    use services::*;
    use ws::ws_handler;

    Router::new()
        .route("/", get(root))
        .nest("/auth", configure_auth_routes())
        .nest("/users", configure_user_routes(state.clone()))
        .nest("/chats", configure_chat_routes(state.clone()))
        .nest("/invitations", configure_invitation_routes(state.clone()))
        .route(
            "/ws",
            any(ws_handler).layer(middleware::from_fn_with_state(
                state.clone(),
                authentication_middleware,
            )),
        )
        .with_state(state)
}

/// Configura le routes di autenticazione (login, register)
fn configure_auth_routes() -> Router<Arc<AppState>> {
    use services::*;
    Router::new()
        .route("/login", post(login_user))
        .route("/register", post(register_user))
}

/// Configura le routes per la gestione degli utenti
fn configure_user_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    use core::authentication_middleware;
    use services::*;
    
    Router::new()
        .route("/", get(search_user_with_username))
        .route("/{user_id}", get(get_user_by_id))
        .route("/me", delete(delete_my_account))
        .layer(middleware::from_fn_with_state(
            state,
            authentication_middleware,
        ))
}

/// Configura le routes per la gestione delle chat
fn configure_chat_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    use core::{authentication_middleware, chat_membership_middleware};
    use services::*;
    
    // Rotte che NON richiedono membership (solo autenticazione)
    let public_routes = Router::new()
        .route("/", get(list_chats).post(create_chat))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authentication_middleware,
        ));

    // Rotte che richiedono membership (autenticazione + membership middleware)
    let member_routes = Router::new()
        .route("/{chat_id}/messages", get(get_chat_messages))
        .route("/{chat_id}/members", get(list_chat_members))
        .route("/{chat_id}/invite/{user_id}", post(invite_to_chat))
        .route(
            "/{chat_id}/members/{user_id}/role",
            patch(update_member_role),
        )
        .route("/{chat_id}/transfer_ownership/{new_owner_id}", patch(transfer_ownership))
        .route("/{chat_id}/members/{user_id}", delete(remove_member))
        .route("/{chat_id}/leave", post(leave_chat))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            chat_membership_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state,
            authentication_middleware,
        ));

    public_routes.merge(member_routes)
}

/// Configura le routes per la gestione degli inviti
fn configure_invitation_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    use core::authentication_middleware;
    use services::*;
    
    Router::new()
        .route("/pending", get(list_pending_invitations))
        .route("/{invite_id}/{action}", post(respond_to_invitation))
        .layer(middleware::from_fn_with_state(
            state,
            authentication_middleware,
        ))
}
