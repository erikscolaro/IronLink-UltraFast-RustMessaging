//! MessageRepository - Repository per la gestione dei messaggi

use super::{Create, Delete, Read, Update};
use crate::dtos::{CreateMessageDTO, UpdateMessageDTO};
use crate::entities::{Message, MessageType};
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
    pub async fn find_many_by_chat_id(&self, chat_id: &i32) -> Result<Vec<Message>, Error> {
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
    async fn create(&self, data: &CreateMessageDTO) -> Result<Message, Error> {
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

    use sqlx::MySqlPool;

    /// Test generico - esempio di utilizzo di #[sqlx::test]
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "messages")))]
    async fn test_example(_pool: MySqlPool) -> sqlx::Result<()> {
        // Il database è stato creato automaticamente con migrations applicate
        // I fixtures sono stati caricati in ordine: users, chats, messages
        // Implementa qui i tuoi test per MessageRepository
        Ok(())
    }
}
