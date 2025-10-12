//! Integration tests per gli endpoints degli utenti
//!
//! Test per:
//! - GET /users?search=username
//! - GET /users/{user_id}
//! - DELETE /users/me

mod common;

#[cfg(test)]
mod user_tests {
    use sqlx::MySqlPool;

    /// Test generico - esempio di utilizzo di #[sqlx::test]
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_example(_pool: MySqlPool) -> sqlx::Result<()> {
        // Il database è stato creato automaticamente con migrations applicate
        // I fixtures users.sql sono stati caricati (alice, bob, charlie disponibili)
        // Implementa qui i tuoi test per gli endpoint degli utenti
        Ok(())
    }
}
