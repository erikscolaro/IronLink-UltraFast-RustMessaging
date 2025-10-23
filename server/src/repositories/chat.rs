//! ChatRepository - Repository per la gestione delle chat

use super::{Create, Delete, Read, Update};
use crate::dtos::{CreateChatDTO, UpdateChatDTO};
use crate::entities::{Chat, ChatType};
use sqlx::{Error, MySqlPool};
use tracing::{debug, info, instrument};

// CHAT REPOSITORY
pub struct ChatRepository {
    connection_pool: MySqlPool,
}

impl ChatRepository {
    pub fn new(connection_pool: MySqlPool) -> Self {
        Self { connection_pool }
    }

    /// Get private chat between two users (if exists)
    /// Optimized query: uses GROUP BY + HAVING instead of multiple JOINs for better performance
    #[instrument(skip(self), fields(user1 = %user1_id, user2 = %user2_id))]
    pub async fn get_private_chat_between_users(
        &self,
        user1_id: &i32,
        user2_id: &i32,
    ) -> Result<Option<Chat>, Error> {
        debug!("Finding private chat between two users");
        let chat = sqlx::query_as!(
            Chat,
            r#"
            SELECT 
                c.chat_id,
                c.title,
                c.description,
                c.chat_type as "chat_type: ChatType"
            FROM chats c
            INNER JOIN userchatmetadata ucm ON c.chat_id = ucm.chat_id
            WHERE c.chat_type = 'PRIVATE' 
            AND ucm.user_id IN (?, ?)
            GROUP BY c.chat_id, c.title, c.description, c.chat_type
            HAVING COUNT(DISTINCT ucm.user_id) = 2
            "#,
            user1_id,
            user2_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        if chat.is_some() {
            info!("Private chat found");
        } else {
            debug!("No private chat found");
        }

        Ok(chat)
    }
}

impl Create<Chat, CreateChatDTO> for ChatRepository {
    #[instrument(skip(self, data), fields(chat_type = ?data.chat_type))]
    async fn create(&self, data: &CreateChatDTO) -> Result<Chat, Error> {
        debug!("Creating new chat");
        // Insert chat using MySQL syntax
        let result = sqlx::query!(
            r#"
            INSERT INTO chats (title, description, chat_type) 
            VALUES (?, ?, ?)
            "#,
            data.title,
            data.description,
            data.chat_type
        )
        .execute(&self.connection_pool)
        .await?;

        // Get the last inserted ID
        let new_id = result.last_insert_id() as i32;

        info!("Chat created with id {}", new_id);

        // Return the created chat with the new ID
        Ok(Chat {
            chat_id: new_id,
            title: data.title.clone(),
            description: data.description.clone(),
            chat_type: data.chat_type.clone(),
        })
    }
}

impl Read<Chat, i32> for ChatRepository {
    #[instrument(skip(self), fields(chat_id = %id))]
    async fn read(&self, id: &i32) -> Result<Option<Chat>, Error> {
        debug!("Reading chat by id");
        let chat = sqlx::query_as!(
            Chat,
            r#"
            SELECT 
                chat_id,
                title,
                description,
                chat_type as "chat_type: ChatType"
            FROM chats 
            WHERE chat_id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        if chat.is_some() {
            debug!("Chat found");
        } else {
            debug!("Chat not found");
        }

        Ok(chat)
    }
}

impl Update<Chat, UpdateChatDTO, i32> for ChatRepository {
    #[instrument(skip(self, data), fields(chat_id = %id))]
    async fn update(&self, id: &i32, data: &UpdateChatDTO) -> Result<Chat, Error> {
        debug!("Updating chat");
        // First, get the current chat to ensure it exists
        let current_chat = self
            .read(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // If no fields to update, return current chat
        if data.title.is_none() && data.description.is_none() {
            debug!("No fields to update, returning current chat");
            return Ok(current_chat);
        }

        // Build dynamic UPDATE query using QueryBuilder (idiomatic SQLx way)
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE chats SET ");

        let mut separated = query_builder.separated(", ");
        if let Some(ref title) = data.title {
            separated.push("title = ");
            separated.push_bind_unseparated(title);
        }
        if let Some(ref description) = data.description {
            separated.push("description = ");
            separated.push_bind_unseparated(description);
        }

        query_builder.push(" WHERE chat_id = ");
        query_builder.push_bind(id);

        query_builder.build().execute(&self.connection_pool).await?;

        info!("Chat updated successfully");

        // Fetch and return the updated chat
        self.read(id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
    }
}

impl Delete<i32> for ChatRepository {
    #[instrument(skip(self), fields(chat_id = %id))]
    async fn delete(&self, id: &i32) -> Result<(), Error> {
        debug!("Deleting chat");
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", id)
            .execute(&self.connection_pool)
            .await?;

        info!("Chat deleted successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::ChatType;
    use sqlx::MySqlPool;

    /*------------------------------------------- */
    /* Unit tests: get_private_chat_between_users */
    /*------------------------------------------- */

    /// Test: trova una chat privata esistente tra due utenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_between_users_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Alice (user_id=1) e Bob (user_id=2) hanno una chat privata (chat_id=2)
        let result = repo.get_private_chat_between_users(&1, &2).await?;
        
        assert!(result.is_some());
        let chat = result.unwrap();
        assert_eq!(chat.chat_id, 2);
        assert_eq!(chat.chat_type, ChatType::Private);
        assert_eq!(chat.title, Some("Private Alice-Bob".to_string()));
        
        Ok(())
    }

    /// Test: l'ordine degli utenti non influisce sul risultato
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_between_users_order_independent(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Cerca prima con (alice, bob)
        let result1 = repo.get_private_chat_between_users(&1, &2).await?;
        
        // Cerca poi con (bob, alice)
        let result2 = repo.get_private_chat_between_users(&2, &1).await?;
        
        assert!(result1.is_some());
        assert!(result2.is_some());
        assert_eq!(result1.unwrap().chat_id, result2.unwrap().chat_id);
        
        Ok(())
    }

    /// Test: non trova chat quando non esiste tra i due utenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_between_users_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Bob (user_id=2) e Charlie (user_id=3) non hanno chat privata
        let result = repo.get_private_chat_between_users(&2, &3).await?;
        
        assert!(result.is_none());
        
        Ok(())
    }

    /// Test: non trova chat GROUP quando si cerca PRIVATE
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_between_users_ignores_group_chats(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Alice (user_id=1) e Bob (user_id=2) sono entrambi nella "General Chat" (GROUP)
        // Ma questo metodo deve trovare solo chat PRIVATE
        // Hanno già una chat privata (chat_id=2), quindi il test verifica che restituisca quella
        let result = repo.get_private_chat_between_users(&1, &2).await?;
        
        assert!(result.is_some());
        let chat = result.unwrap();
        // Deve essere la chat privata, non la group chat
        assert_eq!(chat.chat_type, ChatType::Private);
        assert_eq!(chat.chat_id, 2);
        
        Ok(())
    }

    /// Test: non trova chat con utenti inesistenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_between_users_invalid_users(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Cerca con user_id inesistenti
        let result = repo.get_private_chat_between_users(&999, &1000).await?;
        
        assert!(result.is_none());
        
        Ok(())
    }

    /// Test: gestione con stesso user_id per entrambi i parametri
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_between_users_same_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Cerca chat privata tra lo stesso utente
        let result = repo.get_private_chat_between_users(&1, &1).await?;
        
        // Non dovrebbe trovare nulla (una chat privata richiede 2 utenti distinti)
        assert!(result.is_none());
        
        Ok(())
    }

    /// Test CASCADE: eliminazione di un utente elimina userchatmetadata
    /// La chat rimane ma non è più trovabile con questo metodo
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_cascade_delete_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Verifica che la chat esista
        let result_before = repo.get_private_chat_between_users(&1, &2).await?;
        assert!(result_before.is_some());
        
        // Elimina Alice (user_id=1)
        // CASCADE DELETE eliminerà le righe in userchatmetadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 1)
            .execute(&repo.connection_pool)
            .await?;
        
        // Ora la chat non dovrebbe più essere trovabile
        // perché manca un record in userchatmetadata
        let result_after = repo.get_private_chat_between_users(&1, &2).await?;
        assert!(result_after.is_none());
        
        // Verifica che la chat esista ancora nel database
        let chat_exists = sqlx::query!(
            "SELECT chat_id FROM chats WHERE chat_id = ?",
            2
        )
        .fetch_optional(&repo.connection_pool)
        .await?;
        assert!(chat_exists.is_some(), "La chat dovrebbe esistere ancora");
        
        // Verifica che userchatmetadata sia stato eliminato per user_id=1
        let metadata_count = sqlx::query!(
            "SELECT COUNT(*) as count FROM userchatmetadata WHERE user_id = ?",
            1
        )
        .fetch_one(&repo.connection_pool)
        .await?;
        assert_eq!(metadata_count.count, 0, "Metadata dovrebbe essere eliminato (CASCADE)");
        
        Ok(())
    }

    /// Test CASCADE: eliminazione di una chat elimina userchatmetadata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_cascade_delete_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Verifica che la chat esista
        let result_before = repo.get_private_chat_between_users(&1, &2).await?;
        assert!(result_before.is_some());
        let chat_id = result_before.unwrap().chat_id;
        
        // Elimina la chat
        // CASCADE DELETE eliminerà anche userchatmetadata
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", chat_id)
            .execute(&repo.connection_pool)
            .await?;
        
        // La chat non dovrebbe più essere trovabile
        let result_after = repo.get_private_chat_between_users(&1, &2).await?;
        assert!(result_after.is_none());
        
        // Verifica che userchatmetadata sia stato eliminato
        let metadata_count = sqlx::query!(
            "SELECT COUNT(*) as count FROM userchatmetadata WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&repo.connection_pool)
        .await?;
        assert_eq!(metadata_count.count, 0, "Metadata dovrebbe essere eliminato (CASCADE)");
        
        Ok(())
    }

    /// Test CASCADE: verifica che l'eliminazione di un solo utente
    /// non rompa la query (dovrebbe restituire None)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_cascade_partial_metadata(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Elimina Bob (user_id=2) - CASCADE elimina il suo userchatmetadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&repo.connection_pool)
            .await?;
        
        // La chat privata ha ora solo 1 membro invece di 2
        // La query HAVING COUNT(DISTINCT ucm.user_id) = 2 non dovrebbe trovare nulla
        let result = repo.get_private_chat_between_users(&1, &2).await?;
        assert!(result.is_none());
        
        // Verifica che rimanga solo 1 record in userchatmetadata per questa chat
        let metadata_count = sqlx::query!(
            "SELECT COUNT(*) as count FROM userchatmetadata WHERE chat_id = ?",
            2
        )
        .fetch_one(&repo.connection_pool)
        .await?;
        assert_eq!(metadata_count.count, 1, "Dovrebbe rimanere solo 1 membro");
        
        Ok(())
    }

    /// Test CASCADE: verifica comportamento con chat che ha messaggi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_get_private_chat_cascade_with_messages(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = ChatRepository::new(pool);
        
        // Inserisci un messaggio nella chat privata
        sqlx::query!(
            "INSERT INTO messages (chat_id, sender_id, content, created_at) VALUES (?, ?, ?, NOW())",
            2, 1, "Test message"
        )
        .execute(&repo.connection_pool)
        .await?;
        
        // La chat dovrebbe essere trovabile normalmente
        let result_before = repo.get_private_chat_between_users(&1, &2).await?;
        assert!(result_before.is_some());
        
        // Elimina la chat - CASCADE elimina anche i messaggi
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 2)
            .execute(&repo.connection_pool)
            .await?;
        
        // Verifica che i messaggi siano stati eliminati
        let message_count = sqlx::query!(
            "SELECT COUNT(*) as count FROM messages WHERE chat_id = ?",
            2
        )
        .fetch_one(&repo.connection_pool)
        .await?;
        assert_eq!(message_count.count, 0, "Messaggi dovrebbero essere eliminati (CASCADE)");
        
        Ok(())
    }

    /// Test generico - esempio di utilizzo di #[sqlx::test]
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_example(_pool: MySqlPool) -> sqlx::Result<()> {
        // Il database è stato creato automaticamente con migrations applicate
        // I fixtures sono stati caricati in ordine: users, chats
        // Implementa qui i tuoi test per ChatRepository
        Ok(())
    }
}
