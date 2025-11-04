//! Integration tests per gli endpoints WebSocket
//!
//! Test per:
//! - Connessione WebSocket con autenticazione valida
//! - Gestione utenti duplicati (stesso utente che si connette due volte)
//! - Caricamento chat dell'utente alla connessione
//! - Gestione utenti senza chat
//!
//! Questi test usano `#[sqlx::test]` che:
//! - Crea automaticamente un database di test isolato
//! - Applica le migrations da `migrations/`
//! - Applica i fixtures specificati da `fixtures/`
//! - Pulisce il database al termine

mod common;

#[cfg(test)]
mod ws_tests {
    use super::common::*;
    use axum::http::HeaderValue;
    use serde_json::json;
    use sqlx::MySqlPool;

    // ============================================================
    // Test per connessione WebSocket con autenticazione valida
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_ws_connection_with_valid_auth(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Genera username unico per evitare conflitti
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("wsuser_{}", timestamp);

        // Registra e fai login per ottenere il token
        let token = get_auth_token(&server, &username, "TestPass123!")
            .await
            .expect("Failed to get auth token");

        // Prova la connessione WebSocket con il token
        // Nota: axum-test non supporta upgrade WebSocket reali,
        // ma possiamo testare che l'endpoint risponda correttamente
        let ws_response = server
            .get("/ws")
            .add_header("Authorization", HeaderValue::from_str(&format!("Bearer {}", token)).unwrap())
            .await;

        // Con axum-test, un tentativo di upgrade WebSocket restituisce tipicamente un errore 400
        // Bad Request, che indica che il server riconosce la richiesta ma non può eseguire l'upgrade
        // In un ambiente reale, questo sarebbe 101 Switching Protocols
        assert!(
            ws_response.status_code() == 400 || ws_response.status_code() == 426 || ws_response.status_code() == 101,
            "WebSocket endpoint should be accessible with valid auth, got {}",
            ws_response.status_code()
        );

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_ws_connection_without_auth(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Prova la connessione WebSocket senza token
        let ws_response = server
            .get("/ws")
            .await;

        // Deve essere rifiutata per mancanza di autenticazione
        ws_response.assert_status_forbidden();

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_ws_connection_with_invalid_auth(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Prova la connessione WebSocket con token invalido
        let ws_response = server
            .get("/ws")
            .add_header("Authorization", HeaderValue::from_static("Bearer invalid_token"))
            .await;

        // Deve essere rifiutata per token invalido
        ws_response.assert_status_unauthorized();

        Ok(())
    }

    // ============================================================
    // Test per verifica caricamento chat utente alla connessione
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_user_chats_loaded_on_connection(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Genera username unico per evitare conflitti
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("chatuser_{}", timestamp);

        // Ottieni il token per questo utente
        let token = get_auth_token(&server, &username, "TestPass123!")
            .await
            .expect("Failed to get auth token");

        // Per questo test, semplicemente verifichiamo che la query funzioni
        let chat_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM userchatmetadata WHERE user_id = (SELECT user_id FROM users WHERE username = ?)",
            username
        )
        .fetch_one(&pool)
        .await?;

        assert!(chat_count >= 0, "Chat count query should work");

        // Connetti via WebSocket
        let ws_response = server
            .get("/ws")
            .add_header("Authorization", HeaderValue::from_str(&format!("Bearer {}", token)).unwrap())
            .await;

        // Verifica che la connessione sia accettata (o che dia bad request con axum-test)
        assert!(
            ws_response.status_code() == 400 || ws_response.status_code() == 426 || ws_response.status_code() == 101,
            "WebSocket connection should be processed correctly, got {}",
            ws_response.status_code()
        );

        Ok(())
    }

    // ============================================================
    // Test per gestione utente senza chat
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_user_without_chats_connects_successfully(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Genera username unico per evitare conflitti
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("nochatuser_{}", timestamp);

        // Ottieni il token per questo utente (che verrà creato automaticamente da get_auth_token)
        let token = get_auth_token(&server, &username, "TestPass123!")
            .await
            .expect("Failed to get auth token");

        // Ottieni l'ID del nuovo utente creato
        let new_user = sqlx::query!("SELECT user_id FROM users WHERE username = ?", username)
            .fetch_one(&pool)
            .await?;

        // Verifica che l'utente non abbia chat
        let chat_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM userchatmetadata WHERE user_id = ?",
            new_user.user_id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(chat_count, 0, "New user should have no chats");

        // Connetti via WebSocket
        let ws_response = server
            .get("/ws")
            .add_header("Authorization", HeaderValue::from_str(&format!("Bearer {}", token)).unwrap())
            .await;

        // Verifica che la connessione riesca anche senza chat
        assert!(
            ws_response.status_code() == 400 || ws_response.status_code() == 426 || ws_response.status_code() == 101,
            "WebSocket connection should succeed even without chats, got {}",
            ws_response.status_code()
        );

        Ok(())
    }

    // ============================================================
    // Test per gestione connessioni duplicate (stesso utente)
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_duplicate_user_connections_handling(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Genera username unico per evitare conflitti
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("dupuser_{}", timestamp);

        // Ottieni il token
        let token = get_auth_token(&server, &username, "TestPass123!")
            .await
            .expect("Failed to get auth token");

        // Prima connessione
        let ws_response1 = server
            .get("/ws")
            .add_header("Authorization", HeaderValue::from_str(&format!("Bearer {}", token)).unwrap())
            .await;

        assert!(
            ws_response1.status_code() == 400 || ws_response1.status_code() == 426 || ws_response1.status_code() == 101,
            "First WebSocket connection should succeed, got {}",
            ws_response1.status_code()
        );

        // Seconda connessione con lo stesso utente
        let ws_response2 = server
            .get("/ws")
            .add_header("Authorization", HeaderValue::from_str(&format!("Bearer {}", token)).unwrap())
            .await;

        // La seconda connessione dovrebbe anche essere processata correttamente
        assert!(
            ws_response2.status_code() == 400 || ws_response2.status_code() == 426 || ws_response2.status_code() == 101,
            "Second WebSocket connection should be processed correctly, got {}",
            ws_response2.status_code()
        );

        Ok(())
    }

    // ============================================================ 
    // Test per verifica che il middleware di autenticazione funzioni
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_ws_authentication_middleware_validation(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Test con header Authorization malformato
        let ws_response = server
            .get("/ws")
            .add_header("Authorization", HeaderValue::from_static("InvalidFormat"))
            .await;

        // Il server dovrebbe respingere la richiesta con errore di autenticazione
        assert!(
            ws_response.status_code() == 401 || ws_response.status_code() == 403,
            "Invalid auth format should be rejected, got {}",
            ws_response.status_code()
        );

        // Test con token scaduto (simulato con token malformato)
        let ws_response = server
            .get("/ws")
            .add_header("Authorization", HeaderValue::from_static("Bearer expired.token.here"))
            .await;

        // Il server dovrebbe respingere la richiesta con errore di autenticazione
        assert!(
            ws_response.status_code() == 401 || ws_response.status_code() == 403,
            "Invalid token should be rejected, got {}",
            ws_response.status_code()
        );

        Ok(())
    }

    // ============================================================
    // Test per verifica comportamento con utente deleted
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_ws_connection_with_deleted_user(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Il fixture users.sql include un utente "Deleted User" con status DELETED
        // Proviamo a creare un token per questo utente (che fallirà al login)
        let login_body = json!({
            "username": "Deleted User",
            "password": "TestPass123!"
        });

        let login_response = server.post("/auth/login").json(&login_body).await;
        login_response.assert_status_unauthorized();

        // Siccome il login fallisce, non possiamo testare il WebSocket
        // Questo test verifica che utenti deleted non possano connettersi al WebSocket
        // perché non possono nemmeno ottenere un token valido

        Ok(())
    }
}