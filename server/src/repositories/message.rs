//! MessageRepository - Repository per la gestione dei messaggi

use crate::entities::{Message, MessageType};
use super::Crud;
use chrono::{DateTime, Utc};
use sqlx::{Error, MySqlPool};

// MESSAGE REPO
pub struct MessageRepository {
    connection_pool: MySqlPool,
}

impl MessageRepository {
    pub fn new(connection_pool: MySqlPool) -> Self {
        Self { connection_pool }
    }

    /// Get all messages for a specific chat, ordered by creation time
    pub async fn get_messages_by_chat_id(&self, chat_id: &i32) -> Result<Vec<Message>, Error> {
        let messages = sqlx::query_as!(
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
            ORDER BY created_at ASC
            "#,
            chat_id
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(messages)
    }

    //MOD: imposta timestamp  after_timestamp = current_time-20minuti?????
    /// Get messages for a chat after a specific timestamp (for pagination)
    pub async fn get_messages_after_timestamp(
        &self,
        chat_id: &i32,
        after_timestamp: &DateTime<Utc>,
    ) -> Result<Vec<Message>, Error> {
        let messages = sqlx::query_as!(
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
            WHERE chat_id = ? AND created_at > ?
            ORDER BY created_at ASC
            "#,
            chat_id,
            after_timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(messages)
    }

    //MOD: se vogliomano caricare solo fino a 50 (limit) messaggi
    /// Get messages for a chat with limit (for pagination)
    pub async fn get_messages_with_limit(
        &self,
        chat_id: &i32,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, Error> {
        let messages = sqlx::query_as!(
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
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
            chat_id,
            limit,
            offset
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(messages)
    }
}

impl Crud<Message, crate::dtos::CreateMessageDTO, i32> for MessageRepository {
    async fn create(&self, data: &crate::dtos::CreateMessageDTO) -> Result<Message, Error> {
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

    async fn update(&self, item: &Message) -> Result<Message, Error> {
        sqlx::query!(
            r#"
            UPDATE messages 
            SET chat_id = ?, sender_id = ?, content = ?, message_type = ?, created_at = ?
            WHERE message_id = ?
            "#,
            item.chat_id,
            item.sender_id,
            item.content,
            item.message_type,
            item.created_at,
            item.message_id
        )
        .execute(&self.connection_pool)
        .await?;

        // Return the updated message
        Ok(item.clone())
    }

    async fn delete(&self, id: &i32) -> Result<(), Error> {
        sqlx::query!("DELETE FROM messages WHERE message_id = ?", id)
            .execute(&self.connection_pool)
            .await?;

        Ok(())
    }
}
