//! MessageRepository - Repository per la gestione dei messaggi

use super::{Create, Delete, Read, Update};
use crate::dtos::{CreateMessageDTO, UpdateMessageDTO};
use crate::entities::{Message, MessageType};
use chrono::{DateTime, Utc};
use sqlx::{Error, MySqlPool};
use tracing::{debug, info, instrument};

// MESSAGE REPO
pub struct MessageRepository {
    connection_pool: MySqlPool,
}

impl MessageRepository {
    pub fn new(connection_pool: MySqlPool) -> Self {
        Self { connection_pool }
    }

    /// Get paginated messages for a chat within a time range
    ///
    /// Retrieves messages visible to a user based on their `messages_visible_from` timestamp
    /// (from UserChatMetadata). Supports both:
    /// - Loading recent messages (when `before_date` is None): gets the most recent `limit` messages
    /// - Loading older messages (when `before_date` is Some): gets `limit` messages before that date
    ///
    /// # Arguments
    /// * `chat_id` - The chat ID
    /// * `messages_visible_from` - Lower bound timestamp (from UserChatMetadata.messages_visible_from)
    /// * `before_date` - Optional upper bound timestamp for pagination
    /// * `limit` - Maximum number of messages to return
    ///
    /// # Returns
    /// Messages ordered from newest to oldest (DESC), limited to `limit` count
    pub async fn find_many_paginated(
        &self,
        chat_id: &i32,
        messages_visible_from: &DateTime<Utc>,
        before_date: Option<&DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<Message>, Error> {
        let messages = if let Some(before) = before_date {
            sqlx::query_as!(
                Message,
                r#"
                SELECT 
                    message_id, 
                    chat_id, 
                    sender_id, 
                    content, 
                    created_at,
                    message_type as "message_type: MessageType"
                FROM messages 
                WHERE chat_id = ? 
                  AND created_at >= ? 
                  AND created_at < ?
                ORDER BY created_at DESC
                LIMIT ?
                "#,
                chat_id,
                messages_visible_from,
                before,
                limit
            )
            .fetch_all(&self.connection_pool)
            .await?
        } else {
            sqlx::query_as!(
                Message,
                r#"
                SELECT 
                    message_id, 
                    chat_id, 
                    sender_id, 
                    content, 
                    created_at,
                    message_type as "message_type: MessageType"
                FROM messages 
                WHERE chat_id = ? 
                  AND created_at >= ?
                ORDER BY created_at DESC
                LIMIT ?
                "#,
                chat_id,
                messages_visible_from,
                limit
            )
            .fetch_all(&self.connection_pool)
            .await?
        };

        Ok(messages)
    }
}

impl Create<Message, CreateMessageDTO> for MessageRepository {
    #[instrument(skip(self, data), fields(chat_id = %data.chat_id, sender_id = %data.sender_id))]
    async fn create(&self, data: &CreateMessageDTO) -> Result<Message, Error> {
        debug!("Creating new message");
        // Insert message using MySQL syntax
        let result = sqlx::query!(
            r#"
            INSERT INTO messages (chat_id, sender_id, content, message_type, created_at) 
            VALUES (?, ?, ?, ?, ?)
            "#,
            data.chat_id,
            data.sender_id,
            data.content,
            &data.message_type,
            data.created_at
        )
        .execute(&self.connection_pool)
        .await?;

        // Get the last inserted ID
        let new_id = result.last_insert_id() as i32;

        info!("Message created with id {}", new_id);

        // Return the created message with the new ID
        Ok(Message {
            message_id: new_id,
            chat_id: data.chat_id,
            sender_id: data.sender_id,
            content: data.content.clone(),
            created_at: data.created_at,
            message_type: data.message_type.clone(),
        })
    }
}

impl Read<Message, i32> for MessageRepository {
    async fn read(&self, id: &i32) -> Result<Option<Message>, Error> {
        let message = sqlx::query_as!(
            Message,
            r#"
            SELECT 
                message_id, 
                chat_id, 
                sender_id, 
                content, 
                created_at,
                message_type as "message_type: MessageType"
            FROM messages 
            WHERE message_id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(message)
    }
}

impl Update<Message, UpdateMessageDTO, i32> for MessageRepository {
    async fn update(&self, id: &i32, data: &UpdateMessageDTO) -> Result<Message, Error> {
        // First, get the current message to ensure it exists
        let current_message = self
            .read(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // If no content to update, return current message
        if data.content.is_none() {
            return Ok(current_message);
        }

        // Update message content
        sqlx::query!(
            "UPDATE messages SET content = ? WHERE message_id = ?",
            data.content,
            id
        )
        .execute(&self.connection_pool)
        .await?;

        // Fetch and return the updated message
        self.read(id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
    }
}

impl Delete<i32> for MessageRepository {
    async fn delete(&self, id: &i32) -> Result<(), Error> {
        sqlx::query!("DELETE FROM messages WHERE message_id = ?", id)
            .execute(&self.connection_pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    
    use sqlx::{MySqlPool};
    use super::*;
    use crate::entities::MessageType;
    use chrono::{DateTime, Utc};

    //------------------------------
    //TESTS FOR find_many_paginated
    //------------------------------

    #[sqlx::test]
    async fn test_find_many_paginated_recent_messages_without_before_date(pool: MySqlPool) -> sqlx::Result<()> {

        // Setup: Crea i dati di test manualmente
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password'), (2, 'bob', 'password'), (3, 'charlie', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'General Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW()), (2, 1, NOW(), NOW(), 'MEMBER', NOW()), (3, 1, NOW(), NOW(), 'MEMBER', NOW())")
            .execute(&pool)
            .await?;
        
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES 
            (1, 1, 1, 'Hello everyone!', 'USERMESSAGE', NOW() - INTERVAL 10 MINUTE),
            (2, 1, 2, 'Hi Alice!', 'USERMESSAGE', NOW() - INTERVAL 9 MINUTE),
            (3, 1, 3, 'Good morning!', 'USERMESSAGE', NOW() - INTERVAL 8 MINUTE)")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        // Testa il recupero dei messaggi più recenti senza before_date
        let messages_visible_from = DateTime::from_timestamp(0, 0).unwrap(); // Epoca Unix per vedere tutti i messaggi
        let limit = 10;
        
        let messages = repo.find_many_paginated(&1, &messages_visible_from, None, limit).await?;
        
        // Verifica che ci siano 3 messaggi
        assert_eq!(messages.len(), 3);
        
        // Verifica che tutti appartengano alla chat corretta
        for message in &messages {
            assert_eq!(message.chat_id, 1);
        }
        
        // Verifica l'ordinamento (il più recente dovrebbe essere primo)
        // I messaggi con created_at più alta dovrebbero venire prima
        if messages.len() >= 2 {
            assert!(messages[0].created_at >= messages[1].created_at);
        }
        if messages.len() >= 3 {
            assert!(messages[1].created_at >= messages[2].created_at);
        }
        
        // Il messaggio più recente dovrebbe essere quello con INTERVAL 8 MINUTE (più vicino a NOW)
        assert_eq!(messages[0].content, "Good morning!");
        assert_eq!(messages[1].content, "Hi Alice!");
        assert_eq!(messages[2].content, "Hello everyone!");
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_many_paginated_with_before_date_filter(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup: Crea i dati di test manualmente
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password'), (2, 'bob', 'password'), (3, 'charlie', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'General Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW()), (2, 1, NOW(), NOW(), 'MEMBER', NOW()), (3, 1, NOW(), NOW(), 'MEMBER', NOW())")
            .execute(&pool)
            .await?;
        
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES 
            (1, 1, 1, 'Hello everyone!', 'USERMESSAGE', NOW() - INTERVAL 10 MINUTE),
            (2, 1, 2, 'Hi Alice!', 'USERMESSAGE', NOW() - INTERVAL 9 MINUTE),
            (3, 1, 3, 'Good morning!', 'USERMESSAGE', NOW() - INTERVAL 8 MINUTE)")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        // Prima recupera tutti i messaggi per ottenere una data di riferimento
        let messages_visible_from = DateTime::from_timestamp(0, 0).unwrap();
        let all_messages = repo.find_many_paginated(&1, &messages_visible_from, None, 10).await?;
        
        // Usa la data del secondo messaggio più recente come before_date
        let before_date = all_messages[1].created_at;
        
        let filtered_messages = repo.find_many_paginated(&1, &messages_visible_from, Some(&before_date), 10).await?;
        
        // Dovrebbe restituire solo i messaggi precedenti alla data specificata (1 messaggio)
        assert_eq!(filtered_messages.len(), 1);
        assert_eq!(filtered_messages[0].content, "Hello everyone!");
        
        // Verifica che tutti i messaggi siano anteriori alla data specificata
        for message in &filtered_messages {
            assert!(message.created_at < before_date);
        }
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_many_paginated_with_messages_visible_from_filter(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup: Crea i dati di test manualmente
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password'), (2, 'bob', 'password'), (3, 'charlie', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'General Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW()), (2, 1, NOW(), NOW(), 'MEMBER', NOW()), (3, 1, NOW(), NOW(), 'MEMBER', NOW())")
            .execute(&pool)
            .await?;
        
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES 
            (1, 1, 1, 'Hello everyone!', 'USERMESSAGE', NOW() - INTERVAL 10 MINUTE),
            (2, 1, 2, 'Hi Alice!', 'USERMESSAGE', NOW() - INTERVAL 9 MINUTE),
            (3, 1, 3, 'Good morning!', 'USERMESSAGE', NOW() - INTERVAL 8 MINUTE)")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        // Usa una data nel futuro come messages_visible_from per simulare un utente
        // che è entrato nella chat dopo tutti i messaggi
        let future_date = Utc::now() + chrono::Duration::minutes(5);
        
        let messages = repo.find_many_paginated(&1, &future_date, None, 10).await?;
        
        // Non dovrebbe restituire alcun messaggio perché tutti sono precedenti a messages_visible_from
        assert_eq!(messages.len(), 0);
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_many_paginated_with_limit(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup: Crea i dati di test manualmente
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password'), (2, 'bob', 'password'), (3, 'charlie', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'General Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW()), (2, 1, NOW(), NOW(), 'MEMBER', NOW()), (3, 1, NOW(), NOW(), 'MEMBER', NOW())")
            .execute(&pool)
            .await?;
        
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES 
            (1, 1, 1, 'Hello everyone!', 'USERMESSAGE', NOW() - INTERVAL 10 MINUTE),
            (2, 1, 2, 'Hi Alice!', 'USERMESSAGE', NOW() - INTERVAL 9 MINUTE),
            (3, 1, 3, 'Good morning!', 'USERMESSAGE', NOW() - INTERVAL 8 MINUTE)")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        let messages_visible_from = DateTime::from_timestamp(0, 0).unwrap();
        let limit = 2; // Limita a 2 messaggi
        
        let messages = repo.find_many_paginated(&1, &messages_visible_from, None, limit).await?;
        
        // Verifica che il limite sia rispettato
        assert_eq!(messages.len(), 2);
        
        // Verifica che siano i 2 più recenti
        assert_eq!(messages[0].content, "Good morning!");
        assert_eq!(messages[1].content, "Hi Alice!");
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_many_paginated_nonexistent_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = MessageRepository::new(pool);
        
        let messages_visible_from = DateTime::from_timestamp(0, 0).unwrap();
        let nonexistent_chat_id = 999;
        
        let messages = repo.find_many_paginated(&nonexistent_chat_id, &messages_visible_from, None, 10).await?;
        
        // Non dovrebbe restituire alcun messaggio per una chat inesistente
        assert_eq!(messages.len(), 0);
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_many_paginated_empty_chat(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup: Crea una chat senza messaggi
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (999, 'Empty Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 999, NOW(), NOW(), 'OWNER', NOW())")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        let messages_visible_from = DateTime::from_timestamp(0, 0).unwrap();
        
        let messages = repo.find_many_paginated(&999, &messages_visible_from, None, 10).await?;
        
        // Chat vuota dovrebbe restituire array vuoto
        assert_eq!(messages.len(), 0);
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_many_paginated_message_type_preservation(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup: Crea i dati di test manualmente
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'General Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW())")
            .execute(&pool)
            .await?;
        
        // Inserisci messaggi di diversi tipi
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES 
            (1, 1, 1, 'User message', 'USERMESSAGE', NOW() - INTERVAL 10 MINUTE),
            (2, 1, 1, 'System message test', 'SYSTEMMESSAGE', NOW() - INTERVAL 5 MINUTE)")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        let messages_visible_from = DateTime::from_timestamp(0, 0).unwrap();
        let messages = repo.find_many_paginated(&1, &messages_visible_from, None, 10).await?;
        
        assert_eq!(messages.len(), 2);
        
        // Verifica che ci sia almeno un messaggio di sistema e uno utente
        let system_messages: Vec<_> = messages.iter()
            .filter(|m| m.message_type == MessageType::SystemMessage)
            .collect();
        let user_messages: Vec<_> = messages.iter()
            .filter(|m| m.message_type == MessageType::UserMessage)
            .collect();
        
        assert_eq!(system_messages.len(), 1);
        assert_eq!(user_messages.len(), 1);
        assert_eq!(system_messages[0].content, "System message test");
        assert_eq!(user_messages[0].content, "User message");
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_many_paginated_cascade_behavior_on_chat_deletion(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup: Crea i dati di test manualmente
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password'), (2, 'bob', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (2, 'Test Chat', 'PRIVATE')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 2, NOW(), NOW(), 'OWNER', NOW()), (2, 2, NOW(), NOW(), 'MEMBER', NOW())")
            .execute(&pool)
            .await?;
        
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES 
            (4, 2, 1, 'Test message 1', 'USERMESSAGE', NOW() - INTERVAL 5 MINUTE),
            (5, 2, 2, 'Test message 2', 'USERMESSAGE', NOW() - INTERVAL 4 MINUTE)")
            .execute(&pool)
            .await?;
        
        // Clona il pool prima di passarlo al repository
        let repo = MessageRepository::new(pool.clone());
        
        // Prima verifica che ci siano messaggi per la chat 2
        let messages_visible_from = DateTime::from_timestamp(0, 0).unwrap();
        let messages_before = repo.find_many_paginated(&2, &messages_visible_from, None, 10).await?;
        assert_eq!(messages_before.len(), 2);
        
        // Elimina la chat 2
        sqlx::query!("DELETE FROM chats WHERE chat_id = 2")
            .execute(&pool)
            .await?;
        
        // Verifica che i messaggi siano stati eliminati automaticamente
        let messages_after = repo.find_many_paginated(&2, &messages_visible_from, None, 10).await?;
        assert_eq!(messages_after.len(), 0);
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_many_paginated_cascade_behavior_on_user_deletion(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup: Crea i dati di test manualmente
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password'), (2, 'bob', 'password'), (3, 'charlie', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'General Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW()), (2, 1, NOW(), NOW(), 'MEMBER', NOW()), (3, 1, NOW(), NOW(), 'MEMBER', NOW())")
            .execute(&pool)
            .await?;
        
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES 
            (1, 1, 1, 'Alice message', 'USERMESSAGE', NOW() - INTERVAL 10 MINUTE),
            (2, 1, 2, 'Bob message', 'USERMESSAGE', NOW() - INTERVAL 9 MINUTE),
            (3, 1, 3, 'Charlie message', 'USERMESSAGE', NOW() - INTERVAL 8 MINUTE)")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new((pool).clone());
        
        // Prima verifica che ci siano messaggi dell'utente 2 
        let messages_visible_from = DateTime::from_timestamp(0, 0).unwrap();
        let all_messages_before = repo.find_many_paginated(&1, &messages_visible_from, None, 10).await?;
        let bob_messages_before: Vec<_> = all_messages_before.iter()
            .filter(|m| m.sender_id == 2)
            .collect();
        assert_eq!(bob_messages_before.len(), 1);
        assert_eq!(bob_messages_before[0].content, "Bob message");
        
        // Elimina l'utente bob 
        sqlx::query!("DELETE FROM users WHERE user_id = 2")
            .execute(&pool)
            .await?;
        
        // Verifica che i messaggi di bob siano stati eliminati automaticamente
        let all_messages_after = repo.find_many_paginated(&1, &messages_visible_from, None, 10).await?;
        let bob_messages_after: Vec<_> = all_messages_after.iter()
            .filter(|m| m.sender_id == 2)
            .collect();
        assert_eq!(bob_messages_after.len(), 0);
        
        // Verifica che gli altri messaggi siano ancora presenti
        assert_eq!(all_messages_after.len(), 2);
        
        Ok(())
    }


    //------------------------------
    //TESTS FOR create
    //------------------------------
    #[sqlx::test]
    async fn test_create_message_success(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup: Crea i dati di test necessari
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'Test Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW())")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        // Crea un DTO per il nuovo messaggio
        let create_dto = CreateMessageDTO {
            chat_id: 1,
            sender_id: 1,
            content: "Test message content".to_string(),
            message_type: MessageType::UserMessage,
            created_at: Utc::now(),
        };
        
        // Testa la creazione
        let created_message = repo.create(&create_dto).await?;
        
        // Verifica che il messaggio sia stato creato correttamente
        assert!(created_message.message_id > 0);
        assert_eq!(created_message.chat_id, create_dto.chat_id);
        assert_eq!(created_message.sender_id, create_dto.sender_id);
        assert_eq!(created_message.content, create_dto.content);
        assert_eq!(created_message.message_type, create_dto.message_type);
        assert_eq!(created_message.created_at, create_dto.created_at);
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_system_message(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'system', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'Test Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW())")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        let create_dto = CreateMessageDTO {
            chat_id: 1,
            sender_id: 1,
            content: "User joined the chat".to_string(),
            message_type: MessageType::SystemMessage,
            created_at: Utc::now(),
        };
        
        let created_message = repo.create(&create_dto).await?;
        
        assert_eq!(created_message.message_type, MessageType::SystemMessage);
        assert_eq!(created_message.content, "User joined the chat");
        
        Ok(())
    }


    //------------------------------
    //TESTS FOR read
    //------------------------------
    #[sqlx::test]
    async fn test_read_existing_message(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'Test Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW())")
            .execute(&pool)
            .await?;
        
        // Inserisci un messaggio direttamente
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES (1, 1, 1, 'Test message', 'USERMESSAGE', NOW())")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        // Testa la lettura
        let message = repo.read(&1).await?;
        
        assert!(message.is_some());
        let message = message.unwrap();
        assert_eq!(message.message_id, 1);
        assert_eq!(message.chat_id, 1);
        assert_eq!(message.sender_id, 1);
        assert_eq!(message.content, "Test message");
        assert_eq!(message.message_type, MessageType::UserMessage);
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_read_nonexistent_message(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = MessageRepository::new(pool);
        
        // Testa la lettura di un messaggio inesistente
        let message = repo.read(&999).await?;
        
        assert!(message.is_none());
        
        Ok(())
    }

    //------------------------------
    //TESTS FOR update
    //------------------------------
    #[sqlx::test]
    async fn test_update_message_content(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'Test Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW())")
            .execute(&pool)
            .await?;
        
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES (1, 1, 1, 'Original message', 'USERMESSAGE', NOW())")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        // Crea DTO per l'aggiornamento
        let update_dto = UpdateMessageDTO {
            content: Some("Updated message content".to_string()),
        };
        
        // Testa l'aggiornamento
        let updated_message = repo.update(&1, &update_dto).await?;
        
        assert_eq!(updated_message.message_id, 1);
        assert_eq!(updated_message.content, "Updated message content");
        assert_eq!(updated_message.chat_id, 1);
        assert_eq!(updated_message.sender_id, 1);
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_message_with_none_content(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'Test Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW())")
            .execute(&pool)
            .await?;
        
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES (1, 1, 1, 'Original message', 'USERMESSAGE', NOW())")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool);
        
        // DTO con content = None (nessun aggiornamento)
        let update_dto = UpdateMessageDTO {
            content: None,
        };
        
        // Testa l'aggiornamento con None
        let updated_message = repo.update(&1, &update_dto).await?;
        
        // Il messaggio dovrebbe rimanere invariato
        assert_eq!(updated_message.content, "Original message");
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_nonexistent_message(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = MessageRepository::new(pool);
        
        let update_dto = UpdateMessageDTO {
            content: Some("New content".to_string()),
        };
        
        // Testa l'aggiornamento di un messaggio inesistente
        let result = repo.update(&999, &update_dto).await;
        
        assert!(result.is_err());
        match result {
            Err(sqlx::Error::RowNotFound) => {}, // Comportamento atteso
            _ => panic!("Expected RowNotFound error"),
        }
        
        Ok(())
    }

    //------------------------------
    //TESTS FOR delete
    //------------------------------
    #[sqlx::test]
    async fn test_delete_existing_message(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'Test Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW())")
            .execute(&pool)
            .await?;
        
        sqlx::query!("INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES (1, 1, 1, 'Message to delete', 'USERMESSAGE', NOW())")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool.clone());
        
        // Verifica che il messaggio esista prima della cancellazione
        let message_before = repo.read(&1).await?;
        assert!(message_before.is_some());
        
        // Testa la cancellazione
        let result = repo.delete(&1).await;
        assert!(result.is_ok());
        
        // Verifica che il messaggio sia stato eliminato
        let message_after = repo.read(&1).await?;
        assert!(message_after.is_none());
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_nonexistent_message(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = MessageRepository::new(pool);
        
        // Testa la cancellazione di un messaggio inesistente
        // Dovrebbe completarsi senza errori (operazione idempotente)
        let result = repo.delete(&999).await;
        assert!(result.is_ok());
        
        Ok(())
    }


    //------------------------------
    //TESTS FOR cascade deletions behavior
    //------------------------------
    #[sqlx::test]
    async fn test_crud_cascade_behavior_on_chat_deletion(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'Test Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW())")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool.clone());
        
        // Crea un messaggio tramite CRUD
        let create_dto = CreateMessageDTO {
            chat_id: 1,
            sender_id: 1,
            content: "Test message".to_string(),
            message_type: MessageType::UserMessage,
            created_at: Utc::now(),
        };
        
        let created_message = repo.create(&create_dto).await?;
        
        // Verifica che il messaggio esista
        let message_before = repo.read(&created_message.message_id).await?;
        assert!(message_before.is_some());
        
        // Elimina la chat (dovrebbe attivare CASCADE DELETE)
        sqlx::query!("DELETE FROM chats WHERE chat_id = 1")
            .execute(&pool)
            .await?;
        
        // Verifica che il messaggio sia stato eliminato automaticamente
        let message_after = repo.read(&created_message.message_id).await?;
        assert!(message_after.is_none());
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_crud_cascade_behavior_on_user_deletion(pool: MySqlPool) -> sqlx::Result<()> {
        // Setup
        sqlx::query!("INSERT INTO users (user_id, username, password) VALUES (1, 'alice', 'password'), (2, 'bob', 'password')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO chats (chat_id, title, chat_type) VALUES (1, 'Test Chat', 'GROUP')")
            .execute(&pool)
            .await?;
            
        sqlx::query!("INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES (1, 1, NOW(), NOW(), 'OWNER', NOW()), (2, 1, NOW(), NOW(), 'MEMBER', NOW())")
            .execute(&pool)
            .await?;
        
        let repo = MessageRepository::new(pool.clone());
        
        // Crea messaggi per entrambi gli utenti
        let alice_dto = CreateMessageDTO {
            chat_id: 1,
            sender_id: 1,
            content: "Alice message".to_string(),
            message_type: MessageType::UserMessage,
            created_at: Utc::now(),
        };
        
        let bob_dto = CreateMessageDTO {
            chat_id: 1,
            sender_id: 2,
            content: "Bob message".to_string(),
            message_type: MessageType::UserMessage,
            created_at: Utc::now(),
        };
        
        let alice_message = repo.create(&alice_dto).await?;
        let bob_message = repo.create(&bob_dto).await?;
        
        // Verifica che entrambi i messaggi esistano
        assert!(repo.read(&alice_message.message_id).await?.is_some());
        assert!(repo.read(&bob_message.message_id).await?.is_some());
        
        // Elimina l'utente Bob (dovrebbe attivare CASCADE DELETE sui suoi messaggi)
        sqlx::query!("DELETE FROM users WHERE user_id = 2")
            .execute(&pool)
            .await?;
        
        // Verifica che il messaggio di Bob sia stato eliminato
        assert!(repo.read(&bob_message.message_id).await?.is_none());
        
        // Verifica che il messaggio di Alice sia ancora presente
        assert!(repo.read(&alice_message.message_id).await?.is_some());
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_message_with_invalid_foreign_keys(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = MessageRepository::new(pool);
        
        // Tenta di creare un messaggio con chat_id e sender_id inesistenti
        let create_dto = CreateMessageDTO {
            chat_id: 999, // Chat inesistente
            sender_id: 999, // Utente inesistente
            content: "Test message".to_string(),
            message_type: MessageType::UserMessage,
            created_at: Utc::now(),
        };
        
        // Dovrebbe fallire a causa dei vincoli di foreign key
        let result = repo.create(&create_dto).await;
        assert!(result.is_err());
        
        // Verifica che sia un errore di foreign key constraint
        match result {
            Err(sqlx::Error::Database(db_err)) => {
                // MySQL error code per foreign key constraint violation
                assert!(db_err.message().contains("foreign key constraint") || 
                    db_err.message().contains("Cannot add or update"));
            },
            _ => panic!("Expected foreign key constraint error"),
        }
        
        Ok(())
    }
}
