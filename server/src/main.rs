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

    // Creazione del listener TCP per ascoltare l'indirizzo
    let listener = TcpListener::bind(addr)
        .await
        .expect("Unable to start TCP listener.");

    let root_route = Router::new().route("/", get(root));

    // autenticazione
    let auth_routes = Router::new()
        .route("/login", post(login_user))
        .route(
            "/logout",
            post(logout_user).layer(middleware::from_fn_with_state(
                state.clone(),
                authentication_middleware,
            )),
        )
        .route(
            "/register",
            post(register_user).layer(middleware::from_fn_with_state(
                state.clone(),
                authentication_middleware,
            )),
        );

    // utenti
    let user_routes = Router::new()
        .route("/", get(search_users))
        .route("/:id", get(get_user))
        .route("/me", delete(delete_my_account))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authentication_middleware,
        ));

    // chat
    let chat_routes = Router::new()
        .route("/", get(list_chats).post(create_chat))
        .route("/:id/messages", get(get_chat_messages))
        .route("/:id/members", get(list_chat_members))
        .route("/:id/invite", post(invite_to_chat))
        .route("/:id/members/:id/role", patch(update_member_role))
        .route(
            "/:id/members/:id/transfer-ownership",
            patch(transfer_ownership),
        )
        .route("/:id/members/:id", delete(remove_member))
        .route("/:id/leave", post(leave_chat))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authentication_middleware,
        ));

    // router principale con nesting
    let app = Router::new()
        .merge(root_route) // root senza stato
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/chats", chat_routes)
        .with_state(state);

    // NOTA: rimosse le route degli inviti visto che vengono inviati in chat privata come messaggio di sistema

    // Avvia il server
    axum::serve(listener, app)
        .await
        .expect("Error serving the application");
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
