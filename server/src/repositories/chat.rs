//! ChatRepository - Repository per la gestione delle chat

use super::Crud;
use crate::entities::{Chat, ChatType};
use sqlx::{Error, MySqlPool};

// CHAT REPOSITORY
pub struct ChatRepository {
    connection_pool: MySqlPool,
}

impl ChatRepository {
    pub fn new(connection_pool: MySqlPool) -> Self {
        Self { connection_pool }
    }

    /// Get all chats where user is a member
    pub async fn get_chats_by_user(&self, user_id: &i32) -> Result<Vec<Chat>, Error> {
        let chats = sqlx::query_as!(
            Chat,
            r#"
            SELECT 
                c.chat_id,
                c.title,
                c.description,
                c.chat_type as "chat_type: ChatType"
            FROM chats c
            INNER JOIN userchatmetadata ucm ON c.chat_id = ucm.chat_id
            WHERE ucm.user_id = ?
            "#,
            user_id
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(chats)
    }
    //MOD opzione in piu per cercare chat private tra due utenti (possiamo evitare un ud come input)
    /// Get private chat between two users (if exists)
    pub async fn get_private_chat_between_users(
        &self,
        user1_id: &i32,
        user2_id: &i32,
    ) -> Result<Option<Chat>, Error> {
        let chat = sqlx::query_as!(
            Chat,
            r#"
            SELECT DISTINCT
                c.chat_id,
                c.title,
                c.description,
                c.chat_type as "chat_type: ChatType"
            FROM chats c
            INNER JOIN userchatmetadata ucm1 ON c.chat_id = ucm1.chat_id
            INNER JOIN userchatmetadata ucm2 ON c.chat_id = ucm2.chat_id
            WHERE c.chat_type = 'PRIVATE' 
            AND ucm1.user_id = ? 
            AND ucm2.user_id = ?
            AND ucm1.user_id != ucm2.user_id
            "#,
            user1_id,
            user2_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(chat)
    }

    //MOD: assumo title univoco
    /// Get group chat by title
    pub async fn get_groups_by_title(
        &self,
        title_group: &Option<String>,
    ) -> Result<Option<Chat>, Error> {
        let chats = sqlx::query_as!(
            Chat,
            r#"
            SELECT 
                chat_id,
                title,
                description,
                chat_type as "chat_type: ChatType"
            FROM chats 
            WHERE chat_type = 'GROUP' and title = ?
            "#,
            title_group
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(chats)
    }

    //MOD: forse utile per controlli
    /// Check if chat exists and is of specified type
    pub async fn is_chat_type(
        &self,
        chat_id: &i32,
        expected_type: &ChatType,
    ) -> Result<bool, Error> {
        let result = sqlx::query_as!(
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
            chat_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        match result {
            Some(chat) => Ok(&chat.chat_type == expected_type),
            None => Ok(false),
        }
    }

    /// Update chat title and description (for groups)
    pub async fn update_chat_description(
        &self,
        chat_id: &i32,
        description: &Option<String>,
    ) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE chats SET description = ? WHERE chat_id = ?",
            description,
            chat_id
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }
}

impl Crud<Chat, crate::dtos::CreateChatDTO, crate::dtos::UpdateChatDTO, i32> for ChatRepository {
    async fn create(&self, data: &crate::dtos::CreateChatDTO) -> Result<Chat, Error> {
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

        // Return the created chat with the new ID
        Ok(Chat {
            chat_id: new_id,
            title: data.title.clone(),
            description: data.description.clone(),
            chat_type: data.chat_type.clone(),
        })
    }

    async fn read(&self, id: &i32) -> Result<Option<Chat>, Error> {
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

        Ok(chat)
    }

    async fn update(&self, id: &i32, data: &crate::dtos::UpdateChatDTO) -> Result<Chat, Error> {
        // First, get the current chat to ensure it exists
        let current_chat = self
            .read(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // If no fields to update, return current chat
        if data.title.is_none() && data.description.is_none() {
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

        // Fetch and return the updated chat
        self.read(id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
    }

    async fn delete(&self, id: &i32) -> Result<(), Error> {
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", id)
            .execute(&self.connection_pool)
            .await?;

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
