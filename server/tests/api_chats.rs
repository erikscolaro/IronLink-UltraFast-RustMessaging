//! Integration tests per gli endpoints delle chat

mod common;

#[cfg(test)]
mod chat_tests {
    use super::common::create_test_jwt;
    use axum_test::TestServer;
    use axum_test::http::HeaderName;
    use sqlx::MySqlPool;
    use std::sync::Arc;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_get_chats_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, jwt_secret);

        let response = server
            .get("/chats")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_ok();
        let chats: Vec<serde_json::Value> = response.json();
        assert!(!chats.is_empty(), "L'utente dovrebbe avere almeno una chat");

        for chat in &chats {
            assert!(chat.get("chat_id").is_some(), "Ogni chat deve avere un chat_id");
            assert!(chat.get("title").is_some(), "Ogni chat deve avere un title");
            assert!(chat.get("chat_type").is_some(), "Ogni chat deve avere un chat_type");
        }

        Ok(())
    }
}
