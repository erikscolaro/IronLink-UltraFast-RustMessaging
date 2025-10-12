//! UserChatMetadataRepository - Repository per la gestione dei metadati utente-chat

use crate::entities::{UserChatMetadata, UserRole};
use super::Crud;
use chrono::{DateTime, Utc};
use sqlx::{Error, MySqlPool};

// USERCHATMETADATA REPO
pub struct UserChatMetadataRepository {
    connection_pool: MySqlPool,
}

impl UserChatMetadataRepository {
    pub fn new(connection_pool: MySqlPool) -> Self {
        Self { connection_pool }
    }

    /// Get all members of a specific chat
    pub async fn get_members_by_chat(
        &self,
        chat_id: &i32,
    ) -> Result<Vec<UserChatMetadata>, Error> {
        let metadata_list = sqlx::query_as!(
            UserChatMetadata,
            r#"
            SELECT 
                user_id,
                chat_id,
                user_role as "user_role: UserRole",
                member_since,
                messages_visible_from,
                messages_received_until
            FROM userchatmetadata 
            WHERE chat_id = ?
            "#,
            chat_id
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(metadata_list)
    }

    /// Update user role in a chat
    pub async fn update_user_role(
        &self,
        user_id: &i32,
        chat_id: &i32,
        new_role: &Option<UserRole>,
    ) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE userchatmetadata SET user_role = ? WHERE user_id = ? AND chat_id = ?",
            new_role as &Option<UserRole>,
            user_id,
            chat_id
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    /// Update messages received until timestamp
    pub async fn update_messages_received_until(
        &self,
        user_id: &i32,
        chat_id: &i32,
        timestamp: &DateTime<Utc>,
    ) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE userchatmetadata SET messages_received_until = ? WHERE user_id = ? AND chat_id = ?",
            timestamp,
            user_id,
            chat_id
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    /// Check if user is member of chat
    pub async fn is_user_member(&self, user_id: &i32, chat_id: &i32) -> Result<bool, Error> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM userchatmetadata WHERE user_id = ? AND chat_id = ?",
            user_id,
            chat_id
        )
        .fetch_one(&self.connection_pool)
        .await?;

        Ok(count.count > 0)
    }

    /// Check if user has admin or owner role in chat
    pub async fn is_user_admin_or_owner(
        &self,
        user_id: &i32,
        chat_id: &i32,
    ) -> Result<bool, Error> {
        let result = sqlx::query_as!(
            UserChatMetadata,
            r#"
            SELECT 
                user_id,
                chat_id,
                user_role as "user_role: UserRole",
                member_since,
                messages_visible_from,
                messages_received_until
            FROM userchatmetadata 
            WHERE user_id = ? AND chat_id = ?
            "#,
            user_id,
            chat_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        match result {
            Some(metadata) => {
                // Check if user_role exists and is Admin or Owner
                Ok(matches!(metadata.user_role, Some(UserRole::Admin) | Some(UserRole::Owner)))
            }
            None => Ok(false),
        }
    }

    //MOD inutile??
    /// Get chat owner
    pub async fn get_chat_owner(&self, chat_id: &i32) -> Result<Option<i32>, Error> {
        let result = sqlx::query_as!(
            UserChatMetadata,
            r#"
            SELECT 
                user_id,
                chat_id,
                user_role as "user_role: UserRole",
                member_since,
                messages_visible_from,
                messages_received_until
            FROM userchatmetadata 
            WHERE chat_id = ? AND user_role = 'OWNER'
            "#,
            chat_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(result.map(|metadata| metadata.user_id))
    }

    /// Remove user from chat (delete metadata entry)
    pub async fn remove_user_from_chat(
        &self,
        user_id: &i32,
        chat_id: &i32,
    ) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM userchatmetadata WHERE user_id = ? AND chat_id = ?",
            user_id,
            chat_id
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    //MOD: inutile? si puo far tutto con 'update user role'
    /// Transfer ownership from one user to another in a chat
    pub async fn transfer_ownership(
        &self,
        from_user_id: &i32,
        to_user_id: &i32,
        chat_id: &i32,
    ) -> Result<(), Error> {
        // Start a transaction for atomicity
        let mut tx = self.connection_pool.begin().await?;

        // Update the old owner to admin
        sqlx::query!(
            "UPDATE userchatmetadata SET user_role = 'ADMIN' WHERE user_id = ? AND chat_id = ?",
            from_user_id,
            chat_id
        )
        .execute(&mut *tx)
        .await?;

        // Update the new owner
        sqlx::query!(
            "UPDATE userchatmetadata SET user_role = 'OWNER' WHERE user_id = ? AND chat_id = ?",
            to_user_id,
            chat_id
        )
        .execute(&mut *tx)
        .await?;

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    pub async fn find_all_by_user_id(
        &self,
        user_id: &i32,
    ) -> Result<Vec<UserChatMetadata>, Error> {
        let result = sqlx::query_as!(
            UserChatMetadata,
            r#"
        SELECT
            user_id,
            chat_id,
            user_role as "user_role: UserRole",
            member_since,
            messages_visible_from,
            messages_received_until
        FROM userchatmetadata
        WHERE user_id = ?
        "#,
            user_id
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(result)
    }
}

impl Crud<UserChatMetadata, crate::dtos::CreateUserChatMetadataDTO, i32> for UserChatMetadataRepository {
    async fn create(&self, data: &crate::dtos::CreateUserChatMetadataDTO) -> Result<UserChatMetadata, Error> {
        sqlx::query!(
            r#"
            INSERT INTO userchatmetadata (user_id, chat_id, user_role, member_since, messages_visible_from, messages_received_until) 
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            data.user_id,
            data.chat_id,
            data.user_role,
            data.member_since,
            data.messages_visible_from,
            data.messages_received_until
        )
        .execute(&self.connection_pool)
        .await?;

        // Return the created metadata
        Ok(UserChatMetadata {
            user_id: data.user_id,
            chat_id: data.chat_id,
            user_role: data.user_role.clone(),
            member_since: data.member_since,
            messages_visible_from: data.messages_visible_from,
            messages_received_until: data.messages_received_until,
        })
    }

    async fn read(&self, id: &i32) -> Result<Option<UserChatMetadata>, Error> {
        // For UserChatMetadata, we'll interpret the ID as user_id for simplicity
        // In real scenarios, you might want a composite key approach
        let metadata = sqlx::query_as!(
            UserChatMetadata,
            r#"
            SELECT 
                user_id,
                chat_id,
                user_role as "user_role: UserRole",
                member_since,
                messages_visible_from,
                messages_received_until
            FROM userchatmetadata 
            WHERE user_id = ?
            LIMIT 1
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(metadata)
    }

    async fn update(&self, item: &UserChatMetadata) -> Result<UserChatMetadata, Error> {
        sqlx::query!(
            r#"
            UPDATE userchatmetadata 
            SET user_role = ?
            WHERE user_id = ? AND chat_id = ?
            "#,
            item.user_role,
            item.user_id,
            item.chat_id
        )
        .execute(&self.connection_pool)
        .await?;

        // Return the updated metadata
        Ok(item.clone())
    }

    async fn delete(&self, id: &i32) -> Result<(), Error> {
        // Delete all metadata for a user (interpretation of the ID parameter)
        sqlx::query!("DELETE FROM userchatmetadata WHERE user_id = ?", id)
            .execute(&self.connection_pool)
            .await?;

        Ok(())
    }
}
