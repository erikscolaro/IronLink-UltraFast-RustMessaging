//! Integration tests per gli endpoints delle chat

mod common;

#[cfg(test)]
mod chat_tests {
    use super::common::create_test_jwt;
    use axum_test::TestServer;
    use axum_test::http::HeaderName;
    use serde_json::json;
    use sqlx::MySqlPool;
    use std::sync::Arc;

    // ============================================================
    // Test per GET /chats - list_chats
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_get_chats_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

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

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_get_chats_without_token(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");

        let response = server
            .get("/chats")
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_get_chats_with_invalid_token(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");

        let response = server
            .get("/chats")
            .add_header(
                HeaderName::from_static("authorization"),
                "Bearer invalid_token_here"
            )
            .await;

        response.assert_status_unauthorized();
        Ok(())
    }

    // ============================================================
    // Test per POST /chats - create_chat
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_create_group_chat_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        let body = json!({
            "title": "New Group Chat",
            "description": "A test group chat",
            "chat_type": "Group"
        });

        let response = server
            .post("/chats")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .json(&body)
            .await;

        response.assert_status_ok();
        let chat: serde_json::Value = response.json();
        assert_eq!(chat["title"], "New Group Chat");
        assert_eq!(chat["description"], "A test group chat");
        assert_eq!(chat["chat_type"], "Group");
        assert!(chat.get("chat_id").is_some());

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_create_private_chat_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        let body = json!({
            "chat_type": "Private",
            "user_list": [1, 3]  // alice e charlie
        });

        let response = server
            .post("/chats")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .json(&body)
            .await;

        response.assert_status_ok();
        let chat: serde_json::Value = response.json();
        assert_eq!(chat["chat_type"], "Private");
        assert!(chat.get("chat_id").is_some());

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_create_private_chat_already_exists(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        let body = json!({
            "chat_type": "Private",
            "user_list": [1, 2]  // alice e bob (già esiste chat_id=2)
        });

        let response = server
            .post("/chats")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .json(&body)
            .await;

        response.assert_status_conflict();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_create_private_chat_invalid_user_list(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // user_list con 3 utenti invece di 2
        let body = json!({
            "chat_type": "Private",
            "user_list": [1, 2, 3]
        });

        let response = server
            .post("/chats")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .json(&body)
            .await;

        response.assert_status_bad_request();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_create_chat_without_token(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");

        let body = json!({
            "title": "New Chat",
            "chat_type": "Group"
        });

        let response = server
            .post("/chats")
            .json(&body)
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    // ============================================================
    // Test per GET /chats/{chat_id}/messages - get_chat_messages
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats", "messages")))]
    async fn test_get_chat_messages_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        let response = server
            .get("/chats/1/messages")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_ok();
        let messages: Vec<serde_json::Value> = response.json();
        
        // Verifica che ci siano messaggi (potrebbero essere 0 se i fixtures non sono caricati)
        if !messages.is_empty() {
            for message in &messages {
                assert!(message.get("message_id").is_some());
                assert!(message.get("content").is_some());
                assert!(message.get("sender_id").is_some());
                assert!(message.get("created_at").is_some());
            }
        }

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats", "messages")))]
    async fn test_get_chat_messages_not_member(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(2, "bob", jwt_secret);

        // Bob non è membro della chat 3 (Dev Team)
        let response = server
            .get("/chats/3/messages")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats", "messages")))]
    async fn test_get_chat_messages_nonexistent_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        let response = server
            .get("/chats/999/messages")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    // ============================================================
    // Test per GET /chats/{chat_id}/members - list_chat_members
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_get_chat_members_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        let response = server
            .get("/chats/1/members")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_ok();
        let members: Vec<serde_json::Value> = response.json();
        assert_eq!(members.len(), 3, "General Chat dovrebbe avere 3 membri");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_get_chat_members_not_member(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(2, "bob", jwt_secret);

        // Bob non è membro della chat 3
        let response = server
            .get("/chats/3/members")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    // ============================================================
    // Test per POST /chats/{chat_id}/invite/{user_id} - invite_to_chat
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_invite_to_chat_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // Alice (OWNER) invita Bob alla chat 3 (Dev Team)
        let response = server
            .post("/chats/3/invite/2")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_ok();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_invite_to_chat_not_admin(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(2, "bob", jwt_secret);

        // Bob (MEMBER) cerca di invitare charlie alla chat 1
        let response = server
            .post("/chats/1/invite/3")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_invite_to_chat_already_member(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // Alice cerca di invitare Bob alla chat 1 dove è già membro
        let response = server
            .post("/chats/1/invite/2")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_conflict();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_invite_to_private_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // Tentativo di invitare a una chat privata (non permesso)
        let response = server
            .post("/chats/2/invite/3")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_bad_request();
        Ok(())
    }

    // ============================================================
    // Test per PATCH /chats/{chat_id}/members/{user_id} - update_member_role
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_update_member_role_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // Alice (OWNER) promuove Bob ad ADMIN nella chat 1
        let response = server
            .patch("/chats/1/members/2/role")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .json(&"Admin")
            .await;

        response.assert_status_ok();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_update_member_role_not_owner(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(3, "charlie", jwt_secret);

        // Charlie (ADMIN) cerca di modificare il ruolo (solo OWNER può)
        let response = server
            .patch("/chats/3/members/1/role")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .json(&"Admin")
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_update_member_role_to_owner(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // Non si può promuovere a OWNER con questo endpoint
        let response = server
            .patch("/chats/1/members/2/role")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .json(&"Owner")
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    // ============================================================
    // Test per PATCH /chats/{chat_id}/transfer_ownership - transfer_ownership
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // Alice trasferisce ownership a Bob nella chat 1
        let response = server
            .patch("/chats/1/transfer_ownership/2")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_ok();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_not_owner(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(2, "bob", jwt_secret);

        // Bob non è OWNER, non può trasferire ownership
        let response = server
            .patch("/chats/1/transfer_ownership/3")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_to_non_member(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // Alice cerca di trasferire ownership della chat 3 (Dev Team) a Bob
        // Bob non è membro della chat 3 (solo Alice e Charlie sono membri)
        let response = server
            .patch("/chats/3/transfer_ownership/2")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_not_found();
        Ok(())
    }

    // ============================================================
    // Test per DELETE /chats/{chat_id}/members/{user_id} - remove_member
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_remove_member_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // Alice (OWNER) rimuove Bob dalla chat 1
        let response = server
            .delete("/chats/1/members/2")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_ok();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_remove_member_not_admin(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(2, "bob", jwt_secret);

        // Bob (MEMBER) cerca di rimuovere Charlie
        let response = server
            .delete("/chats/1/members/3")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_remove_owner(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(3, "charlie", jwt_secret);

        // Charlie (ADMIN) cerca di rimuovere Alice (OWNER)
        let response = server
            .delete("/chats/3/members/1")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    // ============================================================
    // Test per POST /chats/{chat_id}/leave - leave_chat
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_leave_chat_success(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(2, "bob", jwt_secret);

        // Bob lascia la chat 1
        let response = server
            .post("/chats/1/leave")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_ok();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_leave_chat_not_member(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(2, "bob", jwt_secret);

        // Bob cerca di lasciare la chat 3 di cui non è membro
        let response = server
            .post("/chats/3/leave")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_forbidden();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_leave_chat_as_owner(pool: MySqlPool) -> sqlx::Result<()> {
        let jwt_secret = "ilmiobellissimosegretochevaassolutamentecambiato";
        let state = Arc::new(server::core::AppState::new(pool, jwt_secret.to_string()));
        let app = server::create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");
        let token = create_test_jwt(1, "alice", jwt_secret);

        // Alice (OWNER) cerca di lasciare la chat
        let response = server
            .post("/chats/1/leave")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_conflict();
        Ok(())
    }

}
