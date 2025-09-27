use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod config;
mod handlers;
mod models;
mod repositories;
mod services;

use crate::auth::authentication_middleware;
use crate::services::login_user;
use crate::{
    repositories::{
        ChatRepository, InvitationRepository, MessageRepository, UserChatMetadataRepository,
        UserRepository,
    },
    services::{
        create_chat, delete_my_account, get_chat_messages, get_user, invite_to_chat, leave_chat,
        list_chat_members, list_chats, logout_user, register_user, remove_member, root,
        search_users, transfer_ownership, update_member_role,
    },
};
use axum::{
    Router, middleware,
    routing::{delete, get, patch, post},
};
use dotenv::dotenv;
use sqlx::mysql::MySqlPoolOptions;
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;

struct AppState {
    user: UserRepository,
    chat: ChatRepository,
    msg: MessageRepository,
    invitation: InvitationRepository,
    meta: UserChatMetadataRepository,
    jwt_secret: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inizializza la configurazione
    config::init();

    // Crea il router
    let app = Router::new().merge(routes::router());

    // creiamo una struct repos per poterla condiividere come stato alle varie routes
    let state = Arc::new(AppState {
        user: UserRepository::new(connection_pool.clone()),
        chat: ChatRepository::new(connection_pool.clone()),
        msg: MessageRepository::new(connection_pool.clone()),
        invitation: InvitationRepository::new(connection_pool.clone()),
        meta: UserChatMetadataRepository::new(connection_pool.clone()),
        jwt_secret: env::var("JWT_SECRET").unwrap_or("un segreto meno bello".to_string()),
    });

    // Definizione indirizzo del server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on http://{}", addr);

    // Crea il listener TCP
    let listener = TcpListener::bind(addr).await?;

    // Avvia il server
    axum::serve(listener, app).await?;

    Ok(())
}

// --- TEST MINIMALE ---
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    // for `oneshot`

    #[tokio::test]
    async fn test_root() {
        let app = Router::new().merge(routes::router());
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
