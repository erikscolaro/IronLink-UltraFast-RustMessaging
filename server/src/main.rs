use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod routes;
mod handlers;
mod config;
mod models;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inizializza la configurazione
    config::init();

    // Crea il router
    let app = Router::new().merge(routes::router());

    // Definisci l'indirizzo
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on http://{}", addr);

    // Crea il listener TCP
    let listener = TcpListener::bind(addr).await?;

    // Avvia il server
    axum::serve(listener, app)
        .await?;

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
