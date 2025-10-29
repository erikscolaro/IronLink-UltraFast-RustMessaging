//! Integration tests per gli endpoints di autenticazione
//!
//! Test per:
//! - POST /auth/login
//! - POST /auth/register
//!
//! Questi test usano `#[sqlx::test]` che:
//! - Crea automaticamente un database di test isolato
//! - Applica le migrations da `migrations/`
//! - Applica i fixtures specificati da `fixtures/`
//! - Pulisce il database al termine

mod common;

#[cfg(test)]
mod auth_tests {
    use super::common::*;
    use serde_json::json;
    use sqlx::MySqlPool;

    // ============================================================
    // Test per POST /auth/login - login_user
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_login_success(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        // Prima registriamo un nuovo utente
        let register_body = json!({
            "username": "logintest",
            "password": "TestLogin123"
        });

        let register_response = server.post("/auth/register").json(&register_body).await;

        register_response.assert_status_ok();

        // Poi facciamo login con le stesse credenziali
        let login_body = json!({
            "username": "logintest",
            "password": "TestLogin123"
        });

        let response = server.post("/auth/login").json(&login_body).await;

        response.assert_status_ok();

        // Verifica che ci sia il cookie Set-Cookie
        let headers = response.headers();
        assert!(
            headers.get("set-cookie").is_some(),
            "Set-Cookie header should be present"
        );

        // Verifica che ci sia l'header Authorization
        assert!(
            headers.get("authorization").is_some(),
            "Authorization header should be present"
        );

        let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
        assert!(
            auth_header.starts_with("Bearer "),
            "Authorization should start with 'Bearer '"
        );

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_login_wrong_password(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "alice",
            "password": "wrongpassword"
        });

        let response = server.post("/auth/login").json(&body).await;

        response.assert_status_unauthorized();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_login_nonexistent_user(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "nonexistent",
            "password": "password123"
        });

        let response = server.post("/auth/login").json(&body).await;

        response.assert_status_unauthorized();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_login_deleted_user(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "Deleted User",
            "password": "password123"
        });

        let response = server.post("/auth/login").json(&body).await;

        response.assert_status_unauthorized();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_login_missing_password(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "alice"
        });

        let response = server.post("/auth/login").json(&body).await;

        // 422 Unprocessable Entity quando manca un campo obbligatorio
        response.assert_status_unprocessable_entity();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_login_missing_username(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "password": "password123"
        });

        let response = server.post("/auth/login").json(&body).await;

        // 422 Unprocessable Entity quando manca un campo obbligatorio
        response.assert_status_unprocessable_entity();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_login_empty_body(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({});

        let response = server.post("/auth/login").json(&body).await;

        // 422 Unprocessable Entity quando manca un campo obbligatorio
        response.assert_status_unprocessable_entity();
        Ok(())
    }

    // ============================================================
    // Test per POST /auth/register - register_user
    // ============================================================

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_success(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "newuser",
            "password": "Password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_ok();
        let user: serde_json::Value = response.json();

        assert!(user.get("id").is_some(), "User should have an id");
        assert_eq!(user["username"], "newuser", "Username should match");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_duplicate_username(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "alice",
            "password": "Password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_conflict();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_deleted_user_username(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "Deleted User",
            "password": "Password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_bad_request();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_username_too_short(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "ab",
            "password": "Password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_bad_request();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_username_too_long(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "a".repeat(51),
            "password": "Password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_bad_request();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_username_invalid_characters(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "user@name",
            "password": "Password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_bad_request();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_password_too_short(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "newuser",
            "password": "Pass1"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_bad_request();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_password_no_uppercase(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "newuser",
            "password": "password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_bad_request();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_password_no_lowercase(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "newuser",
            "password": "PASSWORD123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_bad_request();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_password_no_digit(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "newuser",
            "password": "PasswordOnly"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_bad_request();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_missing_username(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "password": "Password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        // 422 Unprocessable Entity quando manca un campo obbligatorio
        response.assert_status_unprocessable_entity();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_missing_password(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "newuser"
        });

        let response = server.post("/auth/register").json(&body).await;

        // 422 Unprocessable Entity quando manca un campo obbligatorio
        response.assert_status_unprocessable_entity();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_empty_body(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({});

        let response = server.post("/auth/register").json(&body).await;

        // 422 Unprocessable Entity quando manca un campo obbligatorio
        response.assert_status_unprocessable_entity();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_valid_username_with_numbers(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "user123",
            "password": "Password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_ok();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_valid_username_with_underscores(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        let body = json!({
            "username": "user_name",
            "password": "Password123"
        });

        let response = server.post("/auth/register").json(&body).await;

        response.assert_status_ok();
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_register_then_login(pool: MySqlPool) -> sqlx::Result<()> {
        let state = create_test_state(pool);
        let server = create_test_server(state.clone());

        // Prima registrazione
        let register_body = json!({
            "username": "testuser",
            "password": "TestPass123"
        });

        let register_response = server.post("/auth/register").json(&register_body).await;

        register_response.assert_status_ok();

        // Poi login con le stesse credenziali
        let login_body = json!({
            "username": "testuser",
            "password": "TestPass123"
        });

        let login_response = server.post("/auth/login").json(&login_body).await;

        login_response.assert_status_ok();

        let headers = login_response.headers();
        assert!(
            headers.get("authorization").is_some(),
            "Should have authorization header after login"
        );

        Ok(())
    }
}
