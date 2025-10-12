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
    use sqlx::MySqlPool;

    /// Test generico - esempio di utilizzo di #[sqlx::test]
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
    async fn test_example(_pool: MySqlPool) -> sqlx::Result<()> {
        // Il database è stato creato automaticamente con migrations applicate
        // I fixtures users.sql sono stati caricati (alice, bob, charlie disponibili)
        // Implementa qui i tuoi test per gli endpoint di autenticazione
        Ok(())
    }
}
