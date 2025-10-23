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
    use super::*;
    use crate::entities::UserRole;
    use sqlx::MySqlPool;

    /*----------------------------------*/
    /* Unit tests: find_many_by_chat_id */
    /*----------------------------------*/

    /// Test: trova tutti i membri di una chat esistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);
        
        // La "General Chat" (chat_id=1) ha 3 membri: alice, bob, charlie
        let result = repo.find_many_by_chat_id(&1).await?;
        
        assert_eq!(result.len(), 3);
        
        // Verifica che tutti gli user_id siano presenti
        let user_ids: Vec<i32> = result.iter().map(|m| m.user_id).collect();
        assert!(user_ids.contains(&1)); // alice
        assert!(user_ids.contains(&2)); // bob
        assert!(user_ids.contains(&3)); // charlie
        
        Ok(())
    }

    /// Test: trova i membri di una chat privata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_private_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);
        
        // La chat privata Alice-Bob (chat_id=2) ha 2 membri
        let result = repo.find_many_by_chat_id(&2).await?;
        
        assert_eq!(result.len(), 2);
        
        let user_ids: Vec<i32> = result.iter().map(|m| m.user_id).collect();
        assert!(user_ids.contains(&1)); // alice
        assert!(user_ids.contains(&2)); // bob
        
        Ok(())
    }

    /// Test: restituisce lista vuota per chat inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);
        
        // Chat inesistente
        let result = repo.find_many_by_chat_id(&999).await?;
        
        assert_eq!(result.len(), 0);
        assert!(result.is_empty());
        
        Ok(())
    }

    /// Test: verifica che i ruoli siano caricati correttamente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_with_roles(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);
        
        // General Chat (chat_id=1): alice=OWNER, bob=MEMBER, charlie=MEMBER
        let result = repo.find_many_by_chat_id(&1).await?;
        
        // Trova alice (user_id=1)
        let alice_metadata = result.iter().find(|m| m.user_id == 1).unwrap();
        assert_eq!(alice_metadata.user_role, Some(UserRole::Owner));
        
        // Trova bob (user_id=2)
        let bob_metadata = result.iter().find(|m| m.user_id == 2).unwrap();
        assert_eq!(bob_metadata.user_role, Some(UserRole::Member));
        
        // Trova charlie (user_id=3)
        let charlie_metadata = result.iter().find(|m| m.user_id == 3).unwrap();
        assert_eq!(charlie_metadata.user_role, Some(UserRole::Member));
        
        Ok(())
    }

    /// Test: verifica i diversi ruoli in una chat (OWNER, ADMIN, MEMBER)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_different_roles(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);
        
        // Dev Team (chat_id=3): alice=OWNER, charlie=ADMIN
        let result = repo.find_many_by_chat_id(&3).await?;
        
        assert_eq!(result.len(), 2);
        
        let alice = result.iter().find(|m| m.user_id == 1).unwrap();
        assert_eq!(alice.user_role, Some(UserRole::Owner));
        
        let charlie = result.iter().find(|m| m.user_id == 3).unwrap();
        assert_eq!(charlie.user_role, Some(UserRole::Admin));
        
        Ok(())
    }

    /// Test CASCADE: eliminazione di un utente elimina i suoi metadata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_cascade_delete_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());
        
        // Prima: General Chat ha 3 membri
        let result_before = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(result_before.len(), 3);
        
        // Elimina Bob (user_id=2)
        // CASCADE DELETE eliminerà i suoi metadata in tutte le chat
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;
        
        // Dopo: General Chat dovrebbe avere solo 2 membri
        let result_after = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(result_after.len(), 2);
        
        // Verifica che Bob non sia più presente
        let user_ids: Vec<i32> = result_after.iter().map(|m| m.user_id).collect();
        assert!(!user_ids.contains(&2)); // bob non c'è più
        assert!(user_ids.contains(&1)); // alice c'è ancora
        assert!(user_ids.contains(&3)); // charlie c'è ancora
        
        Ok(())
    }

    /// Test CASCADE: eliminazione di una chat elimina tutti i metadata associati
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_cascade_delete_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());
        
        // Prima: General Chat (chat_id=1) ha 3 membri
        let result_before = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(result_before.len(), 3);
        
        // Elimina la chat
        // CASCADE DELETE eliminerà tutti i metadata associati
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;
        
        // Dopo: nessun metadata dovrebbe esistere per quella chat
        let result_after = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(result_after.len(), 0);
        assert!(result_after.is_empty());
        
        // Verifica nel database che i metadata siano stati eliminati
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM userchatmetadata WHERE chat_id = ?",
            1
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(count.count, 0, "Tutti i metadata dovrebbero essere eliminati (CASCADE)");
        
        Ok(())
    }

    /// Test CASCADE: eliminazione di utente che è OWNER in più chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_cascade_delete_owner(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());
        
        // Alice (user_id=1) è OWNER in chat 1, 2 e 3
        // Verifica stato iniziale
        let chat1_before = repo.find_many_by_chat_id(&1).await?;
        let chat2_before = repo.find_many_by_chat_id(&2).await?;
        let chat3_before = repo.find_many_by_chat_id(&3).await?;
        
        assert_eq!(chat1_before.len(), 3); // General Chat
        assert_eq!(chat2_before.len(), 2); // Private Alice-Bob
        assert_eq!(chat3_before.len(), 2); // Dev Team
        
        // Elimina Alice
        // CASCADE eliminerà i suoi metadata da tutte le chat
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 1)
            .execute(&pool)
            .await?;
        
        // Verifica che Alice sia stata rimossa da tutte le chat
        let chat1_after = repo.find_many_by_chat_id(&1).await?;
        let chat2_after = repo.find_many_by_chat_id(&2).await?;
        let chat3_after = repo.find_many_by_chat_id(&3).await?;
        
        assert_eq!(chat1_after.len(), 2); // bob e charlie rimangono
        assert_eq!(chat2_after.len(), 1); // solo bob rimane
        assert_eq!(chat3_after.len(), 1); // solo charlie rimane
        
        // Verifica che Alice non sia in nessuna chat
        assert!(!chat1_after.iter().any(|m| m.user_id == 1));
        assert!(!chat2_after.iter().any(|m| m.user_id == 1));
        assert!(!chat3_after.iter().any(|m| m.user_id == 1));
        
        Ok(())
    }

    /// Test: aggiunta di un nuovo membro e verifica che sia trovato
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_after_adding_member(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());
        
        // Dev Team (chat_id=3) inizialmente ha 2 membri: alice e charlie
        let result_before = repo.find_many_by_chat_id(&3).await?;
        assert_eq!(result_before.len(), 2);
        
        // Aggiungi Bob al Dev Team
        sqlx::query!(
            r#"
            INSERT INTO userchatmetadata (user_id, chat_id, user_role, member_since, messages_visible_from, messages_received_until)
            VALUES (?, ?, 'MEMBER', NOW(), NOW(), NOW())
            "#,
            2, 3
        )
        .execute(&pool)
        .await?;
        
        // Ora dovrebbe avere 3 membri
        let result_after = repo.find_many_by_chat_id(&3).await?;
        assert_eq!(result_after.len(), 3);
        
        // Verifica che Bob sia presente
        let bob = result_after.iter().find(|m| m.user_id == 2);
        assert!(bob.is_some());
        assert_eq!(bob.unwrap().user_role, Some(UserRole::Member));
        
        Ok(())
    }

    /// Test: rimozione di un membro specifico
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_after_removing_member(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());
        
        // General Chat (chat_id=1) ha 3 membri
        let result_before = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(result_before.len(), 3);
        
        // Rimuovi Charlie dalla General Chat
        sqlx::query!(
            "DELETE FROM userchatmetadata WHERE user_id = ? AND chat_id = ?",
            3, 1
        )
        .execute(&pool)
        .await?;
        
        // Ora dovrebbe avere 2 membri
        let result_after = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(result_after.len(), 2);
        
        // Verifica che Charlie non ci sia più
        assert!(!result_after.iter().any(|m| m.user_id == 3));
        
        // Ma alice e bob dovrebbero essere ancora presenti
        assert!(result_after.iter().any(|m| m.user_id == 1));
        assert!(result_after.iter().any(|m| m.user_id == 2));
        
        Ok(())
    }

    /// Test: verifica che i timestamp siano caricati correttamente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_with_timestamps(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);
        
        let result = repo.find_many_by_chat_id(&1).await?;
        
        // Verifica che tutti i membri abbiano timestamp validi
        for metadata in result {
            assert!(metadata.member_since > chrono::DateTime::<chrono::Utc>::default());
            assert!(metadata.messages_visible_from > chrono::DateTime::<chrono::Utc>::default());
            assert!(metadata.messages_received_until > chrono::DateTime::<chrono::Utc>::default());
        }
        
        Ok(())
    }

    /// Test CASCADE: eliminazione di più utenti contemporaneamente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_chat_id_cascade_delete_multiple_users(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());
        
        // General Chat (chat_id=1) ha 3 membri
        let result_before = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(result_before.len(), 3);
        
        // Elimina Bob e Charlie
        sqlx::query!("DELETE FROM users WHERE user_id IN (?, ?)", 2, 3)
            .execute(&pool)
            .await?;
        
        // Dovrebbe rimanere solo Alice
        let result_after = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(result_after.len(), 1);
        assert_eq!(result_after[0].user_id, 1);
        assert_eq!(result_after[0].user_role, Some(UserRole::Owner));
        
        Ok(())
    }

    /// Test generico - esempio di utilizzo di #[sqlx::test]
    #[sqlx::test]
    async fn test_example(_pool: MySqlPool) -> sqlx::Result<()> {
        // Il database è stato creato automaticamente con migrations applicate
        // I fixtures sono stati caricati in ordine: users, chats, messages
        // Implementa qui i tuoi test per MessageRepository
        Ok(())
    }
}
