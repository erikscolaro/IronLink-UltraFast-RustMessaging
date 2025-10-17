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
    use sqlx::MySqlPool;

    /// Test generico - esempio di utilizzo di #[sqlx::test]
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_example(_pool: MySqlPool) -> sqlx::Result<()> {
        // Il database è stato creato automaticamente con migrations applicate
        // I fixtures sono stati caricati in ordine: users, chats
        // Implementa qui i tuoi test per ChatRepository
        Ok(())
    }
}
