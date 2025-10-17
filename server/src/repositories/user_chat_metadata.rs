//! UserChatMetadataRepository - Repository per la gestione dei metadati utente-chat

use super::{Create, Delete, Read, Update};
use crate::dtos::{CreateUserChatMetadataDTO, UpdateUserChatMetadataDTO};
use crate::entities::{UserChatMetadata, UserRole};
use sqlx::{Error, MySqlPool};
use tracing::{debug, info, instrument};

// USERCHATMETADATA REPO
pub struct UserChatMetadataRepository {
    connection_pool: MySqlPool,
}

impl UserChatMetadataRepository {
    pub fn new(connection_pool: MySqlPool) -> Self {
        Self { connection_pool }
    }

    /// Get all members of a specific chat
    pub async fn find_many_by_chat_id(
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

    /// Get all chats for a specific user
    pub async fn find_many_by_user_id(
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

    /// Create multiple metadata entries in a single transaction
    /// Ensures atomicity: either all are created or none
    pub async fn create_many(
        &self,
        metadata_list: &[CreateUserChatMetadataDTO],
    ) -> Result<Vec<UserChatMetadata>, Error> {
        if metadata_list.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.connection_pool.begin().await?;

        let mut created = Vec::with_capacity(metadata_list.len());

        for data in metadata_list {
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
            .execute(&mut *tx)
            .await?;

            created.push(UserChatMetadata {
                user_id: data.user_id,
                chat_id: data.chat_id,
                user_role: data.user_role.clone(),
                member_since: data.member_since,
                messages_visible_from: data.messages_visible_from,
                messages_received_until: data.messages_received_until,
            });
        }

        tx.commit().await?;

        Ok(created)
    }

    pub async fn update_user_role(
        &self,
        user_id: &i32,
        chat_id: &i32,
        new_role: &UserRole,
    ) -> Result<UserChatMetadata, Error> {
        // Mappo l'enum sul valore testuale usato in DB
        let role_str = match new_role {
            UserRole::Owner => "OWNER",
            UserRole::Admin => "ADMIN",
            UserRole::Member => "MEMBER",
        };

        // UPDATE mirato su chiave composta (user_id, chat_id)
        let result = sqlx::query!(
            r#"
            UPDATE userchatmetadata
            SET user_role = ?
            WHERE user_id = ? AND chat_id = ?
            "#,
            role_str,
            user_id,
            chat_id
        )
        .execute(&self.connection_pool)
        .await?;

        // Se nessuna riga è stata toccata, la coppia (user_id, chat_id) non esiste
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        // Ritorno il record aggiornato
        self.read(&(*user_id, *chat_id))
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }
}

impl Create<UserChatMetadata, CreateUserChatMetadataDTO> for UserChatMetadataRepository {
    #[instrument(skip(self, data), fields(user_id = %data.user_id, chat_id = %data.chat_id))]
    async fn create(&self, data: &CreateUserChatMetadataDTO) -> Result<UserChatMetadata, Error> {
        debug!("Creating new user chat metadata");
        // Insert metadata using MySQL syntax
        sqlx::query!(
            r#"
            INSERT INTO userchatmetadata 
            (user_id, chat_id, user_role, member_since, messages_visible_from, messages_received_until) 
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

        info!("User chat metadata created for user {} in chat {}", data.user_id, data.chat_id);

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
}

/// Alias per chiarezza: tipo usato come 'ID' composto per le operazioni
/// su `UserChatMetadata`.
/// Convenzione:
/// - `UserChatKey.0` => `user_id`
/// - `UserChatKey.1` => `chat_id`
///
/// Usare questo alias nelle firme di `read`, `update`, `delete` aiuta
/// l'IDE a mostrare la documentazione quando si richiama quelle funzioni.
pub type UserChatKey = (i32, i32);

impl Read<UserChatMetadata, UserChatKey> for UserChatMetadataRepository {
    async fn read(&self, id: &UserChatKey) -> Result<Option<UserChatMetadata>, Error> {
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
            AND chat_id = ?
            "#,
            id.0,
            id.1
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(metadata)
    }
}

impl Update<UserChatMetadata, UpdateUserChatMetadataDTO, UserChatKey>
    for UserChatMetadataRepository
{
    async fn update(
        &self,
        id: &UserChatKey,
        data: &UpdateUserChatMetadataDTO,
    ) -> Result<UserChatMetadata, Error> {
        // First, get the current metadata to ensure it exists
        let current_metadata = self
            .read(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // If no fields to update, return current metadata
        if data.user_role.is_none()
            && data.messages_visible_from.is_none()
            && data.messages_received_until.is_none()
        {
            return Ok(current_metadata);
        }

        // Build dynamic UPDATE query using QueryBuilder (idiomatic SQLx way)
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE userchatmetadata SET ");

        let mut separated = query_builder.separated(", ");
        if let Some(ref role) = data.user_role {
            separated.push("user_role = ");
            separated.push_bind_unseparated(role);
        }
        if let Some(ref visible_from) = data.messages_visible_from {
            separated.push("messages_visible_from = ");
            separated.push_bind_unseparated(visible_from);
        }
        if let Some(ref received_until) = data.messages_received_until {
            separated.push("messages_received_until = ");
            separated.push_bind_unseparated(received_until);
        }

        query_builder.push(" WHERE user_id = ");
        query_builder.push_bind(id.0);

        query_builder.push(" AND chat_id = ");
        query_builder.push_bind(id.1);

        query_builder.build().execute(&self.connection_pool).await?;

        // Fetch and return the updated metadata
        self.read(id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
    }
}

impl Delete<UserChatKey> for UserChatMetadataRepository {
    async fn delete(&self, id: &UserChatKey) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM userchatmetadata WHERE user_id = ? AND chat_id=?",
            id.0,
            id.1
        )
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
        // I fixtures sono stati caricati in ordine: users, chats (con userchatmetadata)
        // Implementa qui i tuoi test per UserChatMetadataRepository
        Ok(())
    }
}
