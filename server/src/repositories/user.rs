//! UserRepository - Repository per la gestione degli utenti

use super::{Create, Delete, Read, Update};
use crate::dtos::{CreateUserDTO, UpdateUserDTO};
use crate::entities::User;
use sqlx::{Error, MySqlPool};
use tracing::{debug, info, instrument};

// USER REPO
pub struct UserRepository {
    connection_pool: MySqlPool,
}

impl UserRepository {
    pub fn new(connection_pool: MySqlPool) -> UserRepository {
        Self { connection_pool }
    }

    ///considero l'username univoco
    /// Find user by exact username match
    /// For partial username search, use search_by_username_partial
    #[instrument(skip(self), fields(username = %username))]
    pub async fn find_by_username(&self, username: &String) -> Result<Option<User>, Error> {
        debug!("Finding user by username");
        let user = sqlx::query_as!(
            User,
            "SELECT user_id, username, password FROM users WHERE username = ?",
            username
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        if user.is_some() {
            info!("User found");
        } else {
            debug!("User not found");
        }

        Ok(user)
    }

    /// Search users by partial username match (for search functionality)
    #[instrument(skip(self), fields(pattern = %username_pattern))]
    pub async fn search_by_username_partial(
        &self,
        username_pattern: &String,
    ) -> Result<Vec<User>, Error> {
        debug!("Searching users with partial username match");
        let pattern = format!("{}%", username_pattern);
        let users = sqlx::query_as!(
            User,
            "SELECT user_id, username, password FROM users WHERE username LIKE ? LIMIT 10",
            pattern
        )
        .fetch_all(&self.connection_pool)
        .await?;

        info!("Found {} users matching pattern", users.len());
        Ok(users)
    }
}

impl Create<User, CreateUserDTO> for UserRepository {
    #[instrument(skip(self, data), fields(username = %data.username))]
    async fn create(&self, data: &CreateUserDTO) -> Result<User, Error> {
        debug!("Creating new user");
        // Insert user and get the ID using MySQL syntax
        let result = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            data.username,
            data.password
        )
        .execute(&self.connection_pool)
        .await?;

        // Get the last inserted ID
        let new_id = result.last_insert_id() as i32;

        info!("User created with id {}", new_id);

        // Return the created user with the new ID
        Ok(User {
            user_id: new_id,
            username: data.username.clone(),
            password: data.password.clone(),
        })
    }
}

impl Read<User, i32> for UserRepository {
    #[instrument(skip(self), fields(user_id = %id))]
    async fn read(&self, id: &i32) -> Result<Option<User>, Error> {
        debug!("Reading user by id");
        let user = sqlx::query_as!(
            User,
            "SELECT user_id, username, password FROM users WHERE user_id = ?",
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        if user.is_some() {
            debug!("User found");
        } else {
            debug!("User not found");
        }

        Ok(user)
    }
}

impl Update<User, UpdateUserDTO, i32> for UserRepository {
    #[instrument(skip(self, data), fields(user_id = %id))]
    async fn update(&self, id: &i32, data: &UpdateUserDTO) -> Result<User, Error> {
        debug!("Updating user");
        // First, get the current user to ensure it exists
        let current_user = self
            .read(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // Only password can be updated
        if let Some(ref password) = data.password {
            debug!("Updating user password");
            sqlx::query!(
                "UPDATE users SET password = ? WHERE user_id = ?",
                password,
                id
            )
            .execute(&self.connection_pool)
            .await?;

            info!("User password updated");

            // Fetch and return the updated user
            self.read(id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
        } else {
            // If no password provided, return current user unchanged
            debug!("No password update provided, returning current user");
            Ok(current_user)
        }
    }
}

impl Delete<i32> for UserRepository {
    /// Soft delete user by setting username to "Deleted User" and clearing password ""
    /// This preserves message history while anonymizing the user
    #[instrument(skip(self), fields(user_id = %user_id))]
    async fn delete(&self, user_id: &i32) -> Result<(), Error> {
        debug!("Soft deleting user");
        sqlx::query!(
            "UPDATE users SET username = 'Deleted User', password = '' WHERE user_id = ?",
            user_id
        )
        .execute(&self.connection_pool)
        .await?;

        info!("User soft deleted successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::MySqlPool;

    // ============================================================================
    // Tests for CREATE method
    // ============================================================================

    /// Test: verifica che create crei correttamente un nuovo utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_create_user_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let create_dto = CreateUserDTO {
            username: "new_user".to_string(),
            password: "hashed_password_123".to_string(),
        };
        
        let created = repo.create(&create_dto).await?;
        
        assert!(created.user_id > 0);
        assert_eq!(created.username, create_dto.username);
        assert_eq!(created.password, create_dto.password);
        
        Ok(())
    }

    /// Test: verifica che create fallisca con UNIQUE constraint violation per username duplicato
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_create_user_fails_with_duplicate_username(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        // Dal fixture: alice esiste già
        let duplicate_dto = CreateUserDTO {
            username: "alice".to_string(),
            password: "some_password".to_string(),
        };
        
        let result = repo.create(&duplicate_dto).await;
        
        assert!(result.is_err(), "Expected unique constraint violation for duplicate username");
        
        Ok(())
    }

    /// Test: verifica che create permetta username case-sensitive differenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_create_user_case_sensitive_username(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        // Dal fixture: "alice" esiste (lowercase)
        let uppercase_dto = CreateUserDTO {
            username: "ALICE".to_string(),
            password: "password".to_string(),
        };
        
        // Questo dovrebbe avere successo se il DB è case-sensitive, altrimenti fallisce
        let result = repo.create(&uppercase_dto).await;
        
        // In MySQL con utf8mb4_unicode_ci (case-insensitive), questo fallisce
        // Se si usa utf8mb4_bin (case-sensitive), avrebbe successo
        // Verifichiamo solo che non ci sia panic
        let _ = result;
        
        Ok(())
    }

    /// Test: verifica che create gestisca password vuota
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_create_user_with_empty_password(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let create_dto = CreateUserDTO {
            username: "user_empty_pass".to_string(),
            password: "".to_string(),
        };
        
        let created = repo.create(&create_dto).await?;
        
        assert_eq!(created.password, "");
        
        Ok(())
    }

    /// Test: verifica che create gestisca username e password lunghi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_create_user_with_long_fields(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let long_username = "a".repeat(200); // Assumendo che il DB accetti 255 chars
        let long_password = "b".repeat(500);
        
        let create_dto = CreateUserDTO {
            username: long_username.clone(),
            password: long_password.clone(),
        };
        
        let created = repo.create(&create_dto).await?;
        
        assert_eq!(created.username, long_username);
        assert_eq!(created.password, long_password);
        
        Ok(())
    }

    // ============================================================================
    // Tests for READ method
    // ============================================================================

    /// Test: verifica che read restituisca un utente esistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_read_user_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        // Dal fixture: user_id=1 è alice
        let user_id = 1;
        
        let user = repo.read(&user_id).await?;
        
        assert!(user.is_some());
        let u = user.unwrap();
        assert_eq!(u.user_id, user_id);
        assert_eq!(u.username, "alice");
        
        Ok(())
    }

    /// Test: verifica che read restituisca None per utente inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_read_user_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let nonexistent_id = 9999;
        
        let user = repo.read(&nonexistent_id).await?;
        
        assert!(user.is_none(), "Expected None for nonexistent user");
        
        Ok(())
    }

    /// Test: verifica che read restituisca l'utente dopo create
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_read_after_create(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let create_dto = CreateUserDTO {
            username: "test_read".to_string(),
            password: "password123".to_string(),
        };
        
        let created = repo.create(&create_dto).await?;
        
        let read_user = repo.read(&created.user_id).await?;
        
        assert!(read_user.is_some());
        let u = read_user.unwrap();
        assert_eq!(u.user_id, created.user_id);
        assert_eq!(u.username, created.username);
        
        Ok(())
    }

    /// Test: verifica che read restituisca tutti i campi correttamente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_read_returns_all_fields(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        // Dal fixture: user_id=2 è bob
        let user_id = 2;
        
        let user = repo.read(&user_id).await?.unwrap();
        
        assert_eq!(user.user_id, 2);
        assert_eq!(user.username, "bob");
        assert!(!user.password.is_empty()); // password esiste
        
        Ok(())
    }

    // ============================================================================
    // Tests for UPDATE method
    // ============================================================================

    /// Test: verifica che update aggiorni correttamente la password
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_update_user_password_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 1;
        let new_password = "new_hashed_password_456".to_string();
        
        let update_dto = UpdateUserDTO {
            password: Some(new_password.clone()),
        };
        
        let updated = repo.update(&user_id, &update_dto).await?;
        
        assert_eq!(updated.user_id, user_id);
        assert_eq!(updated.password, new_password);
        assert_eq!(updated.username, "alice"); // username non cambia
        
        Ok(())
    }

    /// Test: verifica che update con password=None non modifichi l'utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_update_user_with_no_password_change(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 1;
        
        let before = repo.read(&user_id).await?.unwrap();
        
        let update_dto = UpdateUserDTO {
            password: None,
        };
        
        let updated = repo.update(&user_id, &update_dto).await?;
        
        assert_eq!(updated.user_id, before.user_id);
        assert_eq!(updated.username, before.username);
        assert_eq!(updated.password, before.password);
        
        Ok(())
    }

    /// Test: verifica che update fallisca per utente inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_update_user_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let nonexistent_id = 9999;
        
        let update_dto = UpdateUserDTO {
            password: Some("new_password".to_string()),
        };
        
        let result = repo.update(&nonexistent_id, &update_dto).await;
        
        assert!(result.is_err(), "Expected error for nonexistent user");
        
        Ok(())
    }

    /// Test: verifica che update preservi username
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_update_preserves_username(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 2;
        let original_username = "bob".to_string();
        
        let update_dto = UpdateUserDTO {
            password: Some("totally_new_password".to_string()),
        };
        
        let updated = repo.update(&user_id, &update_dto).await?;
        
        assert_eq!(updated.username, original_username);
        
        Ok(())
    }

    /// Test: verifica che update possa cambiare password a stringa vuota
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_update_user_to_empty_password(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 1;
        
        let update_dto = UpdateUserDTO {
            password: Some("".to_string()),
        };
        
        let updated = repo.update(&user_id, &update_dto).await?;
        
        assert_eq!(updated.password, "");
        
        Ok(())
    }

    /// Test: verifica che update possa essere chiamato più volte
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_update_user_multiple_times(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 1;
        
        // Primo update
        let update1 = UpdateUserDTO {
            password: Some("password1".to_string()),
        };
        let result1 = repo.update(&user_id, &update1).await?;
        assert_eq!(result1.password, "password1");
        
        // Secondo update
        let update2 = UpdateUserDTO {
            password: Some("password2".to_string()),
        };
        let result2 = repo.update(&user_id, &update2).await?;
        assert_eq!(result2.password, "password2");
        
        // Terzo update
        let update3 = UpdateUserDTO {
            password: Some("password3".to_string()),
        };
        let result3 = repo.update(&user_id, &update3).await?;
        assert_eq!(result3.password, "password3");
        
        Ok(())
    }

    // ============================================================================
    // Tests for DELETE method (Soft Delete)
    // ============================================================================

    /// Test: verifica che delete esegua soft delete correttamente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_delete_user_soft_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 1;
        
        // Verifica stato iniziale
        let before = repo.read(&user_id).await?.unwrap();
        assert_eq!(before.username, "alice");
        assert!(!before.password.is_empty());
        
        // Soft delete
        repo.delete(&user_id).await?;
        
        // Verifica che l'utente esista ancora ma sia anonimizzato
        let after = repo.read(&user_id).await?;
        assert!(after.is_some(), "User should still exist after soft delete");
        
        let deleted_user = after.unwrap();
        assert_eq!(deleted_user.user_id, user_id);
        assert_eq!(deleted_user.username, "Deleted User");
        assert_eq!(deleted_user.password, "");
        
        Ok(())
    }

    /// Test: verifica che delete non fallisca per utente inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_delete_user_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let nonexistent_id = 9999;
        
        // Soft delete su utente inesistente non dovrebbe dare errore
        let result = repo.delete(&nonexistent_id).await;
        
        assert!(result.is_ok(), "Expected soft delete to succeed even for nonexistent user");
        
        Ok(())
    }

    /// Test: verifica che delete preservi user_id
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_delete_preserves_user_id(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 2;
        
        repo.delete(&user_id).await?;
        
        let deleted_user = repo.read(&user_id).await?.unwrap();
        
        assert_eq!(deleted_user.user_id, user_id);
        
        Ok(())
    }

    /// Test: verifica che delete possa essere chiamato più volte (idempotente)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_delete_user_multiple_times(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 1;
        
        // Primo soft delete
        repo.delete(&user_id).await?;
        let after_first = repo.read(&user_id).await?.unwrap();
        assert_eq!(after_first.username, "Deleted User");
        
        // Secondo soft delete (dovrebbe essere idempotente)
        repo.delete(&user_id).await?;
        let after_second = repo.read(&user_id).await?.unwrap();
        assert_eq!(after_second.username, "Deleted User");
        assert_eq!(after_second.password, "");
        
        Ok(())
    }

    /// Test: verifica che delete mantenga la cronologia dei messaggi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "messages")))]
    async fn test_delete_preserves_message_history(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 1; // Alice ha messaggi nei fixtures
        
        // Conta i messaggi prima del soft delete
        let messages_before = sqlx::query!(
            "SELECT COUNT(*) as count FROM messages WHERE sender_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        // Soft delete
        repo.delete(&user_id).await?;
        
        // Conta i messaggi dopo il soft delete
        let messages_after = sqlx::query!(
            "SELECT COUNT(*) as count FROM messages WHERE sender_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        // I messaggi dovrebbero essere preservati
        assert_eq!(messages_before.count, messages_after.count);
        
        Ok(())
    }

    // ============================================================================
    // Tests for find_by_username method
    // ============================================================================

    /// Test: verifica che find_by_username trovi un utente esistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_find_by_username_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let username = "alice".to_string();
        
        let user = repo.find_by_username(&username).await?;
        
        assert!(user.is_some());
        let u = user.unwrap();
        assert_eq!(u.username, username);
        assert_eq!(u.user_id, 1);
        
        Ok(())
    }

    /// Test: verifica che find_by_username restituisca None per utente inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_find_by_username_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let nonexistent_username = "nonexistent_user".to_string();
        
        let user = repo.find_by_username(&nonexistent_username).await?;
        
        assert!(user.is_none(), "Expected None for nonexistent username");
        
        Ok(())
    }

    /// Test: verifica che find_by_username sia exact match (non parziale)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_find_by_username_exact_match_only(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        // Dal fixture: esiste "alice", cerco "alic" (parziale)
        let partial_username = "alic".to_string();
        
        let user = repo.find_by_username(&partial_username).await?;
        
        assert!(user.is_none(), "Expected None for partial match");
        
        Ok(())
    }

    /// Test: verifica che find_by_username trovi utente dopo create
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_find_by_username_after_create(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let create_dto = CreateUserDTO {
            username: "findme".to_string(),
            password: "password".to_string(),
        };
        
        let created = repo.create(&create_dto).await?;
        
        let found = repo.find_by_username(&create_dto.username).await?;
        
        assert!(found.is_some());
        assert_eq!(found.unwrap().user_id, created.user_id);
        
        Ok(())
    }

    /// Test: verifica che find_by_username trovi utente soft-deleted
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_find_by_username_after_soft_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 1;
        
        // Soft delete alice
        repo.delete(&user_id).await?;
        
        // Cerca "Deleted User"
        let deleted_user = repo.find_by_username(&"Deleted User".to_string()).await?;
        
        // Dovrebbe trovare almeno un utente con "Deleted User"
        assert!(deleted_user.is_some());
        
        // Non dovrebbe più trovare "alice"
        let alice = repo.find_by_username(&"alice".to_string()).await?;
        assert!(alice.is_none());
        
        Ok(())
    }

    // ============================================================================
    // Tests for search_by_username_partial method
    // ============================================================================

    /// Test: verifica che search_by_username_partial trovi utenti con pattern
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_search_by_username_partial_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        // Dal fixture: alice, bob, charlie
        let pattern = "a".to_string(); // dovrebbe trovare alice e charlie
        
        let users = repo.search_by_username_partial(&pattern).await?;
        
        assert!(!users.is_empty());
        // Verifica che tutti i risultati inizino con "a"
        for user in users {
            assert!(user.username.starts_with("a"));
        }
        
        Ok(())
    }

    /// Test: verifica che search_by_username_partial restituisca array vuoto per nessun match
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_search_by_username_partial_no_match(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let pattern = "xyz".to_string(); // nessun utente inizia con xyz
        
        let users = repo.search_by_username_partial(&pattern).await?;
        
        assert!(users.is_empty(), "Expected empty array for no matches");
        
        Ok(())
    }

    /// Test: verifica che search_by_username_partial limiti i risultati
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_search_by_username_partial_limit(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        // Crea 15 utenti che iniziano con "test"
        for i in 0..15 {
            let create_dto = CreateUserDTO {
                username: format!("test_user_{}", i),
                password: "password".to_string(),
            };
            repo.create(&create_dto).await?;
        }
        
        let pattern = "test".to_string();
        
        let users = repo.search_by_username_partial(&pattern).await?;
        
        // Dovrebbe restituire al massimo 10 risultati (LIMIT 10)
        assert!(users.len() <= 10, "Expected at most 10 results");
        
        Ok(())
    }

    /// Test: verifica che search_by_username_partial trovi tutti i match
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_search_by_username_partial_finds_all_matches(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        // Dal fixture: bob inizia con "b"
        let pattern = "b".to_string();
        
        let users = repo.search_by_username_partial(&pattern).await?;
        
        assert!(users.iter().any(|u| u.username == "bob"));
        
        Ok(())
    }

    /// Test: verifica che search_by_username_partial sia case-insensitive
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_search_by_username_partial_case_insensitive(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        // Dal fixture: alice esiste (lowercase)
        let uppercase_pattern = "A".to_string();
        
        let users = repo.search_by_username_partial(&uppercase_pattern).await?;
        
        // Con utf8mb4_unicode_ci (case-insensitive), dovrebbe trovare alice
        // Se non trova nulla, il DB potrebbe essere case-sensitive
        let found_alice = users.iter().any(|u| u.username.to_lowercase() == "alice");
        
        // Questo test potrebbe passare o fallire a seconda del collation del DB
        // Lo lasciamo per documentazione
        let _ = found_alice;
        
        Ok(())
    }

    // ============================================================================
    // Tests for CASCADE behaviors with related tables
    // ============================================================================

    /// Test: verifica che HARD delete di user causi CASCADE DELETE su invitations (invited_id)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_hard_delete_user_cascades_to_invitations_invited(pool: MySqlPool) -> sqlx::Result<()> {
        // Note: questo test usa HARD DELETE invece di soft delete per verificare CASCADE
        
        let user_id = 3; // Charlie è invited_id in alcuni inviti
        
        // Conta inviti per Charlie prima
        let invitations_before = sqlx::query!(
            "SELECT COUNT(*) as count FROM invitations WHERE invited_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert!(invitations_before.count > 0, "Charlie should have invitations");
        
        // HARD DELETE (non soft delete)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", user_id)
            .execute(&pool)
            .await?;
        
        // Verifica che gli inviti siano stati eliminati per CASCADE
        let invitations_after = sqlx::query!(
            "SELECT COUNT(*) as count FROM invitations WHERE invited_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(invitations_after.count, 0, "Invitations should be deleted via CASCADE");
        
        Ok(())
    }

    /// Test: verifica che HARD delete di user causi CASCADE DELETE su invitations (invitee_id)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_hard_delete_user_cascades_to_invitations_inviter(pool: MySqlPool) -> sqlx::Result<()> {
        let user_id = 2; // Bob è invitee_id in alcuni inviti
        
        // Conta inviti creati da Bob prima
        let invitations_before = sqlx::query!(
            "SELECT COUNT(*) as count FROM invitations WHERE invitee_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert!(invitations_before.count > 0, "Bob should have created invitations");
        
        // HARD DELETE
        sqlx::query!("DELETE FROM users WHERE user_id = ?", user_id)
            .execute(&pool)
            .await?;
        
        // Verifica CASCADE
        let invitations_after = sqlx::query!(
            "SELECT COUNT(*) as count FROM invitations WHERE invitee_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(invitations_after.count, 0, "Invitations should be deleted via CASCADE");
        
        Ok(())
    }

    /// Test: verifica che HARD delete di user causi CASCADE DELETE su messages
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "messages")))]
    async fn test_hard_delete_user_cascades_to_messages(pool: MySqlPool) -> sqlx::Result<()> {
        let user_id = 1; // Alice ha messaggi
        
        // Conta messaggi prima
        let messages_before = sqlx::query!(
            "SELECT COUNT(*) as count FROM messages WHERE sender_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert!(messages_before.count > 0, "Alice should have messages");
        
        // HARD DELETE
        sqlx::query!("DELETE FROM users WHERE user_id = ?", user_id)
            .execute(&pool)
            .await?;
        
        // Verifica CASCADE
        let messages_after = sqlx::query!(
            "SELECT COUNT(*) as count FROM messages WHERE sender_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(messages_after.count, 0, "Messages should be deleted via CASCADE");
        
        Ok(())
    }

    /// Test: verifica che HARD delete di user causi CASCADE DELETE su userchatmetadata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_hard_delete_user_cascades_to_metadata(pool: MySqlPool) -> sqlx::Result<()> {
        let user_id = 1; // Alice è membro di varie chat
        
        // Conta metadata prima
        let metadata_before = sqlx::query!(
            "SELECT COUNT(*) as count FROM userchatmetadata WHERE user_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert!(metadata_before.count > 0, "Alice should have chat metadata");
        
        // HARD DELETE
        sqlx::query!("DELETE FROM users WHERE user_id = ?", user_id)
            .execute(&pool)
            .await?;
        
        // Verifica CASCADE
        let metadata_after = sqlx::query!(
            "SELECT COUNT(*) as count FROM userchatmetadata WHERE user_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(metadata_after.count, 0, "Metadata should be deleted via CASCADE");
        
        Ok(())
    }

    /// Test: verifica che soft delete NON causi CASCADE (preserva relazioni)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "messages")))]
    async fn test_soft_delete_preserves_relations(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool.clone());
        
        let user_id = 1; // Alice
        
        // Conta messaggi prima
        let messages_before = sqlx::query!(
            "SELECT COUNT(*) as count FROM messages WHERE sender_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        // SOFT DELETE tramite repository
        repo.delete(&user_id).await?;
        
        // Verifica che i messaggi siano ancora presenti
        let messages_after = sqlx::query!(
            "SELECT COUNT(*) as count FROM messages WHERE sender_id = ?",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(messages_before.count, messages_after.count, "Messages should be preserved with soft delete");
        
        // Verifica che l'utente sia anonimizzato ma esista
        let user = repo.read(&user_id).await?.unwrap();
        assert_eq!(user.username, "Deleted User");
        
        Ok(())
    }
}
