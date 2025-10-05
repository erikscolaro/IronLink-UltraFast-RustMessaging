mod auth;
mod config;
mod dtos;
mod entities;
mod error_handler;
mod repositories;
mod services;
mod ws_services;

use crate::auth::authentication_middleware;
use crate::dtos::WsEventDTO;
use crate::services::login_user;
use crate::ws_services::ws_handler;
use crate::{
    repositories::{
        ChatRepository, InvitationRepository, MessageRepository, UserChatMetadataRepository,
        UserRepository,
    },
    services::{
        create_chat, delete_my_account, get_chat_messages, get_user_by_id, invite_to_chat,
        leave_chat, list_chat_members, list_chats, register_user, remove_member, root,
        search_user_with_username, transfer_ownership, update_member_role,
    },
};
use axum::routing::any;
use axum::{
    Router, middleware,
    routing::{delete, get, patch, post},
};
use dashmap::DashMap;
use sqlx::mysql::MySqlPoolOptions;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;

struct AppState {
    user: UserRepository,
    chat: ChatRepository,
    msg: MessageRepository,
    invitation: InvitationRepository,
    meta: UserChatMetadataRepository,
    jwt_secret: String,
    users_online: DashMap<i32, Sender<WsEventDTO>>,
}

#[tokio::main]
async fn main() {
    // Carica la configurazione dalle variabili d'ambiente
    let config = config::Config::from_env()
        .expect("Failed to load configuration. Check your .env file.");

    // Stampa info sulla configurazione
    config.print_info();

    // Builder per configurare le connessioni al database
    let pool_options = MySqlPoolOptions::new()
        .max_connections(config.max_connections)
        .max_lifetime(Duration::from_secs(config.connection_lifetime_secs))
        .test_before_acquire(true);

    // Avvio il pool di connessioni al database
    let connection_pool = pool_options
        .connect(&config.database_url)
        .await
        .expect("Error connecting to the database");

    println!("✓ Database connection established");

    // Creiamo una struct repos per poterla condividere come stato alle varie routes
    let state = Arc::new(AppState {
        user: UserRepository::new(connection_pool.clone()),
        chat: ChatRepository::new(connection_pool.clone()),
        msg: MessageRepository::new(connection_pool.clone()),
        invitation: InvitationRepository::new(connection_pool.clone()),
        meta: UserChatMetadataRepository::new(connection_pool.clone()),
        jwt_secret: config.jwt_secret.clone(),
        users_online: Default::default(),
    });

    // Definizione indirizzo del server
    let addr = SocketAddr::from((
        config.server_host.parse::<std::net::IpAddr>()
            .expect("Invalid SERVER_HOST format"),
        config.server_port
    ));
    println!("Server listening on http://{}", addr);

    // Creazione del listener TCP per ascoltare l'indirizzo
    let listener = TcpListener::bind(addr)
        .await
        .expect("Unable to start TCP listener.");

    let root_route = Router::new().route("/", get(root));

    // autenticazione
    let auth_routes = Router::new().route("/login", post(login_user)).route(
        "/register",
        post(register_user).layer(middleware::from_fn_with_state(
            state.clone(),
            authentication_middleware,
        )),
    );

    // utenti
    let user_routes = Router::new()
        .route("/", get(search_user_with_username)) // http: GET users?search=
        .route("/{user_id}", get(get_user_by_id))
        .route("/me", delete(delete_my_account))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authentication_middleware,
        ));

    // chat
    let chat_routes = Router::new()
        .route("/", get(list_chats).post(create_chat))
        .route("/{chat_id}/messages", get(get_chat_messages))
        .route("/{chat_id}/members", get(list_chat_members))
        .route("/{chat_id}/invite/{user_id}", post(invite_to_chat))
        .route(
            "/{chat_id}/members/{user_id}/role",
            patch(update_member_role),
        )
        .route("/{chat_id}/transfer_ownership", patch(transfer_ownership))
        .route("/{chat_id}/members/{user_id}", delete(remove_member))
        .route("/{chat_id}/leave", post(leave_chat))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authentication_middleware,
        ));

    // router principale con nesting
    let app = Router::new()
        .merge(root_route)
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/chats", chat_routes)
        //autenticazione fatta prima di upgrade a ws
        .route(
            "/ws",
            any(ws_handler).layer(middleware::from_fn_with_state(
                state.clone(),
                authentication_middleware,
            )),
        )
        .with_state(state);

    // NOTA: rimosse le route degli inviti visto che vengono inviati in chat privata come messaggio di sistema

    // Avvia il server
    axum::serve(listener, app)
        .await
        .expect("Error serving the application");
}
