mod core;
mod dtos;
mod entities;
mod repositories;
mod services;
mod ws;

use crate::core::{AppState, Config, authentication_middleware};
use crate::services::*;
use crate::ws::ws_handler;
use axum::{
    Router, middleware,
    routing::{any, delete, get, patch, post},
};
use sqlx::mysql::MySqlPoolOptions;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;

/// Configura le routes di autenticazione (login, register)
fn configure_auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login_user))
        .route("/register", post(register_user))
}

/// Configura le routes per la gestione degli utenti
fn configure_user_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
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
    Router::new()
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
            state,
            authentication_middleware,
        ))
}

#[tokio::main]
async fn main() {
    // Carica la configurazione dalle variabili d'ambiente
    let config = Config::from_env().expect("Failed to load configuration. Check your .env file.");

    // Stampa info sulla configurazione
    config.print_info();

    // Builder per configurare le connessioni al database con retry automatico
    let pool_options = MySqlPoolOptions::new()
        .max_connections(config.max_connections)
        .max_lifetime(Duration::from_secs(config.connection_lifetime_secs))
        .acquire_timeout(Duration::from_secs(2)) // Timeout per l'acquisizione di una connessione dal pool
        .test_before_acquire(true);

    // Avvio il pool di connessioni al database con retry automatico ogni 2 secondi
    println!("Attempting to connect to database...");
    let connection_pool = loop {
        match pool_options.clone().connect(&config.database_url).await {
            Ok(pool) => {
                println!("✓ Database connection established successfully!");
                break pool;
            }
            Err(e) => {
                eprintln!("✗ Failed to connect to database: {}", e);
                eprintln!("  Retrying in 2 seconds...");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    };

    // Creiamo lo stato dell'applicazione con i repository e la configurazione
    let state = Arc::new(AppState::new(connection_pool, config.jwt_secret.clone()));

    // Definizione indirizzo del server
    let addr = SocketAddr::from((
        config
            .server_host
            .parse::<std::net::IpAddr>()
            .expect("Invalid SERVER_HOST format"),
        config.server_port,
    ));
    println!("Server listening on http://{}", addr);

    // Creazione del listener TCP per ascoltare l'indirizzo
    let listener = TcpListener::bind(addr)
        .await
        .expect("Unable to start TCP listener.");

    // Costruzione del router principale con tutte le routes
    let app = Router::new()
        .route("/", get(root))
        .nest("/auth", configure_auth_routes())
        .nest("/users", configure_user_routes(state.clone()))
        .nest("/chats", configure_chat_routes(state.clone()))
        .route(
            "/ws",
            any(ws_handler).layer(middleware::from_fn_with_state(
                state.clone(),
                authentication_middleware,
            )),
        )
        .with_state(state);

    // Avvia il server
    axum::serve(listener, app)
        .await
        .expect("Error serving the application");
}
