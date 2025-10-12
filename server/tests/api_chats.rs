//! Integration tests per gli endpoints delle chat
//!
//! Test per:
//! - GET /chats
//! - POST /chats
//! - GET /chats/{chat_id}/messages
//! - GET /chats/{chat_id}/members
//! - POST /chats/{chat_id}/invite/{user_id}
//! - PATCH /chats/{chat_id}/members/{user_id}/role
//! - PATCH /chats/{chat_id}/transfer_ownership
//! - DELETE /chats/{chat_id}/members/{user_id}
//! - POST /chats/{chat_id}/leave

mod common;

#[cfg(test)]
mod chat_tests {
    use sqlx::MySqlPool;

    /// Test generico - esempio di utilizzo di #[sqlx::test]
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats", "messages")))]
    async fn test_example(_pool: MySqlPool) -> sqlx::Result<()> {
        // Il database è stato creato automaticamente con migrations applicate
        // I fixtures sono stati caricati in ordine: users, chats, messages
        // Implementa qui i tuoi test per gli endpoint delle chat
        Ok(())
    }
}
