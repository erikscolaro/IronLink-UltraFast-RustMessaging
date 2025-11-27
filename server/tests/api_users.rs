//! Integration tests per gli endpoints degli utenti
//!
//! Test per:
//! - GET /users?search=username
//! - GET /users/{user_id}
//! - DELETE /users/me

mod common;

#[cfg(test)]
mod user_tests {
    use super::common::*;
    use axum_test::http::HeaderName;
    use serde_json::json;
    use server::repositories::Read;
    use sqlx::MySqlPool;

    // ============================================================
    // Test per GET /users?search=username - search_user_with_username
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_search_users_partial_match(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());
        let token = create_test_jwt(1, "alice", &state.jwt_secret);

        let response = server
            .get("/users?search=char")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await;

        response.assert_status_ok();
        let users: Vec<serde_json::Value> = response.json();

        // Dovrebbe trovare "charlie"
        assert!(
            users.iter().any(|u| u["username"] == "charlie"),
            "Should find charlie"
        );

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_search_users_no_results(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());
        let token = create_test_jwt(1, "alice", &state.jwt_secret);

        let response = server
            .get("/users?search=nonexistent")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await;

        response.assert_status_ok();
        let users: Vec<serde_json::Value> = response.json();

        assert!(users.is_empty(), "Should return empty array for no matches");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_search_users_empty_query(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());
        let token = create_test_jwt(1, "alice", &state.jwt_secret);

        let response = server
            .get("/users?search=")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await;

        response.assert_status_ok();
        let _users: Vec<serde_json::Value> = response.json();

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_search_users_without_token(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        let response = server.get("/users?search=alice").await;

        response.assert_status_forbidden();
        Ok(())
    }

    // ============================================================
    // Test per GET /users/{user_id} - get_user_by_id
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_get_user_by_id_success(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());
        let token = create_test_jwt(1, "alice", &state.jwt_secret);

        let response = server
            .get("/users/2")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await;

        response.assert_status_ok();
        let user: serde_json::Value = response.json();

        assert_eq!(user["id"], 2, "User ID should match");
        assert_eq!(user["username"], "bob", "Username should be bob");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_get_user_by_id_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());
        let token = create_test_jwt(1, "alice", &state.jwt_secret);

        let response = server
            .get("/users/999")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await;

        response.assert_status_ok();
        let user: serde_json::Value = response.json();

        // Dovrebbe restituire null per utente non trovato
        assert!(user.is_null(), "Should return null for non-existent user");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_get_user_by_id_self(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());
        let token = create_test_jwt(1, "alice", &state.jwt_secret);

        let response = server
            .get("/users/1")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await;

        response.assert_status_ok();
        let user: serde_json::Value = response.json();

        assert_eq!(user["id"], 1, "User ID should match");
        assert_eq!(user["username"], "alice", "Username should be alice");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_get_user_by_id_without_token(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        let response = server.get("/users/1").await;

        response.assert_status_forbidden();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_get_user_by_id_with_invalid_token(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        let response = server
            .get("/users/1")
            .add_header(
                HeaderName::from_static("authorization"),
                "Bearer invalid_token",
            )
            .await;

        response.assert_status_unauthorized();
        Ok(())
    }

    // ============================================================
    // Test per DELETE /users/me - delete_my_account
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_delete_account_as_regular_member(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());
        let token = create_test_jwt(2, "bob", &state.jwt_secret);

        // Bob è MEMBER in alcune chat, non owner
        let response = server
            .delete("/users/me")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await;

        response.assert_status_ok();

        // Verifica che ci sia il cookie di logout
        let headers = response.headers();
        assert!(
            headers.get("set-cookie").is_some(),
            "Should have Set-Cookie header"
        );

        let cookie = headers.get("set-cookie").unwrap().to_str().unwrap();
        assert!(
            cookie.contains("Max-Age=0"),
            "Cookie should expire immediately"
        );

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_delete_account_as_owner_with_other_members(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());
        let token = create_test_jwt(1, "alice", &state.clone().jwt_secret);

        // Alice è OWNER della chat 1 (General Chat) con bob e charlie come membri
        // Alice è OWNER della chat 2 (Private Alice-Bob) con bob
        // Alice è OWNER della chat 3 (Dev Team) con charlie come ADMIN

        let response = server
            .delete("/users/me")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await;

        response.assert_status_ok();

        // Verifica 1: Alice è stata soft-deleted
        let alice_user = state.user.read(&1).await?;
        assert!(
            alice_user.is_some(),
            "Alice's user record should still exist"
        );
        assert_eq!(
            alice_user.unwrap().username,
            "Deleted User",
            "Alice should be soft-deleted"
        );

        // Verifica 2: La chat 3 (Dev Team) dovrebbe ancora esistere
        let chat3 = state.chat.read(&3).await?;
        assert!(chat3.is_some(), "Chat 3 should still exist");

        // Verifica 3: Charlie dovrebbe essere diventato OWNER della chat 3
        let charlie_meta = state.meta.read(&(3, 3)).await?;
        assert!(charlie_meta.is_some(), "Charlie should still be in chat 3");
        let charlie_role = charlie_meta.unwrap().user_role;
        assert_eq!(
            charlie_role,
            Some(server::entities::enums::UserRole::Owner),
            "Charlie should now be OWNER of chat 3"
        );

        // Verifica 4: Alice non dovrebbe più essere nei metadata delle chat
        let alice_meta_chat3 = state.meta.read(&(1, 3)).await?;
        assert!(
            alice_meta_chat3.is_none(),
            "Alice should be removed from chat 3 metadata"
        );

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_delete_account_without_chats(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Crea un nuovo utente senza chat
        let register_body = json!({
            "username": "newuser",
            "password": "Password123"
        });

        let register_response = server.post("/auth/register").json(&register_body).await;

        register_response.assert_status_ok();

        // Fai login per ottenere il token
        let login_body = json!({
            "username": "newuser",
            "password": "Password123"
        });

        let login_response = server.post("/auth/login").json(&login_body).await;

        login_response.assert_status_ok();
        let headers = login_response.headers();
        let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
        let token = auth_header.strip_prefix("Bearer ").unwrap();

        // Elimina l'account
        let response = server
            .delete("/users/me")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await;

        response.assert_status_ok();

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_delete_account_without_token(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        let response = server.delete("/users/me").await;

        response.assert_status_forbidden();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_delete_account_with_invalid_token(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        let response = server
            .delete("/users/me")
            .add_header(
                HeaderName::from_static("authorization"),
                "Bearer invalid_token",
            )
            .await;

        response.assert_status_unauthorized();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_delete_account_verify_soft_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());
        let token_bob = create_test_jwt(2, "bob", &state.jwt_secret);

        // Elimina l'account di Bob
        let delete_response = server
            .delete("/users/me")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token_bob),
            )
            .await;

        delete_response.assert_status_ok();

        // Verifica che l'utente sia stato rinominato "Deleted User"
        let token_alice = create_test_jwt(1, "alice", &state.jwt_secret);

        let get_response = server
            .get("/users/2")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token_alice),
            )
            .await;

        get_response.assert_status_ok();
        let user: serde_json::Value = get_response.json();

        assert_eq!(
            user["username"], "Deleted User",
            "Username should be 'Deleted User'"
        );

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_delete_account_cannot_login_after(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(&pool);
        let server = create_test_server(state.clone());

        // Crea e logga un nuovo utente
        let register_body = json!({
            "username": "tempuser",
            "password": "TempPass123"
        });

        server
            .post("/auth/register")
            .json(&register_body)
            .await
            .assert_status_ok();

        let login_body = json!({
            "username": "tempuser",
            "password": "TempPass123"
        });

        let login_response = server.post("/auth/login").json(&login_body).await;
        login_response.assert_status_ok();

        let headers = login_response.headers();
        let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
        let token = auth_header.strip_prefix("Bearer ").unwrap();

        // Elimina l'account
        server
            .delete("/users/me")
            .add_header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", token),
            )
            .await
            .assert_status_ok();

        // Prova a fare login con le vecchie credenziali
        let login_after_delete = server.post("/auth/login").json(&login_body).await;

        // Dovrebbe fallire perché l'utente è stato cancellato
        login_after_delete.assert_status_unauthorized();

        Ok(())
    }
}
