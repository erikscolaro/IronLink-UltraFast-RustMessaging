use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod config;
mod error_handler;
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
async fn main() {
    // avvio dotenv
    dotenv().ok();
    // cerco la variaible per connettermi al database
    let database_url =
        env::var("DATABASE_URL").expect("Error retrieving database url from environment.");

    //builder per configurare le connessioni al database
    let pool_options = MySqlPoolOptions::new()
        .max_connections(1000)
        .max_lifetime(Duration::from_secs(1))
        .test_before_acquire(true);

    // avvio il pool di connessioni al database
    let connection_pool = pool_options
        .connect(database_url.as_str())
        .await
        .expect("Error connecting to the database");

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
