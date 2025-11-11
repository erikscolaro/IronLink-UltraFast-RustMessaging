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

        // Verify old owner exists and has correct role
        let _old_owner = sqlx::query_as!(
            UserChatMetadata,
            r#"SELECT 
                   user_id,
                   chat_id,
                   user_role as "user_role: UserRole",
                   member_since,
                   messages_visible_from,
                   messages_received_until
               FROM userchatmetadata 
               WHERE user_id = ? AND chat_id = ?"#,
            from_user_id,
            chat_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::RowNotFound)?;

        // Verify new owner exists
        let _new_owner = sqlx::query_as!(
            UserChatMetadata,
            r#"SELECT 
                   user_id,
                   chat_id,
                   user_role as "user_role: UserRole",
                   member_since,
                   messages_visible_from,
                   messages_received_until
               FROM userchatmetadata 
               WHERE user_id = ? AND chat_id = ?"#,
            to_user_id,
            chat_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::RowNotFound)?;

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

        info!(
            "User chat metadata created for user {} in chat {}",
            data.user_id, data.chat_id
        );

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
        assert_eq!(
            count.count, 0,
            "Tutti i metadata dovrebbero essere eliminati (CASCADE)"
        );

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
            3,
            1
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
    async fn test_find_many_by_chat_id_cascade_delete_multiple_users(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
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

    /*--------------------------------*/
    /* Unit tests: transfer_ownership */
    /*--------------------------------*/

    /// Test: trasferimento di ownership da un utente ad un altro
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Alice (user_id=1) è OWNER della General Chat (chat_id=1)
        // Bob (user_id=2) è MEMBER della General Chat
        let alice_before = repo.read(&(1, 1)).await?.unwrap();
        let bob_before = repo.read(&(2, 1)).await?.unwrap();

        assert_eq!(alice_before.user_role, Some(UserRole::Owner));
        assert_eq!(bob_before.user_role, Some(UserRole::Member));

        // Trasferisci ownership da Alice a Bob
        repo.transfer_ownership(&1, &2, &1).await?;

        // Dopo il trasferimento:
        // Alice dovrebbe essere ADMIN
        // Bob dovrebbe essere OWNER
        let alice_after = repo.read(&(1, 1)).await?.unwrap();
        let bob_after = repo.read(&(2, 1)).await?.unwrap();

        assert_eq!(alice_after.user_role, Some(UserRole::Admin));
        assert_eq!(bob_after.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test: trasferimento ownership da OWNER ad ADMIN
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_owner_to_admin(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Dev Team (chat_id=3): alice=OWNER, charlie=ADMIN
        let alice_before = repo.read(&(1, 3)).await?.unwrap();
        let charlie_before = repo.read(&(3, 3)).await?.unwrap();

        assert_eq!(alice_before.user_role, Some(UserRole::Owner));
        assert_eq!(charlie_before.user_role, Some(UserRole::Admin));

        // Trasferisci ownership da Alice a Charlie
        repo.transfer_ownership(&1, &3, &3).await?;

        // Alice dovrebbe diventare ADMIN
        // Charlie dovrebbe diventare OWNER
        let alice_after = repo.read(&(1, 3)).await?.unwrap();
        let charlie_after = repo.read(&(3, 3)).await?.unwrap();

        assert_eq!(alice_after.user_role, Some(UserRole::Admin));
        assert_eq!(charlie_after.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test: trasferimento ownership in chat privata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_private_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Private Alice-Bob (chat_id=2): alice=OWNER, bob=MEMBER
        repo.transfer_ownership(&1, &2, &2).await?;

        let alice = repo.read(&(1, 2)).await?.unwrap();
        let bob = repo.read(&(2, 2)).await?.unwrap();

        assert_eq!(alice.user_role, Some(UserRole::Admin));
        assert_eq!(bob.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test: atomicità della transazione - entrambe le operazioni devono avere successo
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_atomicity(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Verifica stato iniziale
        let alice_before = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(alice_before.user_role, Some(UserRole::Owner));

        // Trasferimento valido
        repo.transfer_ownership(&1, &2, &1).await?;

        // Verifica che entrambe le modifiche siano state applicate
        let alice_after = repo.read(&(1, 1)).await?.unwrap();
        let bob_after = repo.read(&(2, 1)).await?.unwrap();

        assert_eq!(alice_after.user_role, Some(UserRole::Admin));
        assert_eq!(bob_after.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test: trasferimento con utente non esistente nel database
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_nonexistent_target(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Stato prima del tentativo
        let alice_before = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(alice_before.user_role, Some(UserRole::Owner));

        // Tentativo di trasferire ownership a un utente inesistente (999)
        // Il database non dovrebbe avere un utente con id 999 in questa chat
        let result = repo.transfer_ownership(&1, &999, &1).await;

        // L'operazione dovrebbe completarsi senza errori anche se l'utente target non esiste
        // perché MySQL UPDATE su righe inesistenti non genera errore
        assert!(result.is_ok());

        // Alice dovrebbe essere diventata ADMIN (prima parte dell'operazione)
        let alice_after = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(alice_after.user_role, Some(UserRole::Admin));

        Ok(())
    }

    /// Test CASCADE: eliminazione del vecchio owner dopo trasferimento
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_cascade_delete_old_owner(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Trasferisci ownership da Alice a Bob nella General Chat
        repo.transfer_ownership(&1, &2, &1).await?;

        // Verifica il trasferimento
        let bob = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(bob.user_role, Some(UserRole::Owner));

        // Elimina Alice (ex-owner, ora admin)
        // CASCADE eliminerà i suoi metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 1)
            .execute(&pool)
            .await?;

        // Alice non dovrebbe più esistere nei metadata
        let alice = repo.read(&(1, 1)).await?;
        assert!(alice.is_none());

        // Bob dovrebbe essere ancora owner
        let bob_after = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(bob_after.user_role, Some(UserRole::Owner));

        // La chat dovrebbe avere 2 membri invece di 3
        let members = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(members.len(), 2);

        Ok(())
    }

    /// Test CASCADE: eliminazione del nuovo owner dopo trasferimento
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_cascade_delete_new_owner(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Trasferisci ownership da Alice a Bob
        repo.transfer_ownership(&1, &2, &1).await?;

        // Verifica
        let bob = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(bob.user_role, Some(UserRole::Owner));

        // Elimina Bob (nuovo owner)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;

        // Bob non dovrebbe più esistere
        let bob_after = repo.read(&(2, 1)).await?;
        assert!(bob_after.is_none());

        // Alice (ora admin) dovrebbe essere ancora presente
        let alice = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(alice.user_role, Some(UserRole::Admin));

        Ok(())
    }

    /// Test CASCADE: eliminazione della chat dopo trasferimento ownership
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_cascade_delete_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Trasferisci ownership
        repo.transfer_ownership(&1, &2, &1).await?;

        // Verifica il trasferimento
        let alice = repo.read(&(1, 1)).await?.unwrap();
        let bob = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(alice.user_role, Some(UserRole::Admin));
        assert_eq!(bob.user_role, Some(UserRole::Owner));

        // Elimina la chat
        // CASCADE eliminerà tutti i metadata associati
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Nessun metadata dovrebbe esistere per questa chat
        let alice_after = repo.read(&(1, 1)).await?;
        let bob_after = repo.read(&(2, 1)).await?;

        assert!(alice_after.is_none());
        assert!(bob_after.is_none());

        // Verifica che la chat non abbia membri
        let members = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(members.len(), 0);

        Ok(())
    }

    /// Test: doppio trasferimento di ownership (A->B->C)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_chain(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Stato iniziale: alice=OWNER, bob=MEMBER, charlie=MEMBER
        // Primo trasferimento: Alice -> Bob
        repo.transfer_ownership(&1, &2, &1).await?;

        let alice_after_first = repo.read(&(1, 1)).await?.unwrap();
        let bob_after_first = repo.read(&(2, 1)).await?.unwrap();

        assert_eq!(alice_after_first.user_role, Some(UserRole::Admin));
        assert_eq!(bob_after_first.user_role, Some(UserRole::Owner));

        // Secondo trasferimento: Bob -> Charlie
        repo.transfer_ownership(&2, &3, &1).await?;

        let bob_after_second = repo.read(&(2, 1)).await?.unwrap();
        let charlie_after_second = repo.read(&(3, 1)).await?.unwrap();

        assert_eq!(bob_after_second.user_role, Some(UserRole::Admin));
        assert_eq!(charlie_after_second.user_role, Some(UserRole::Owner));

        // Alice dovrebbe essere ancora admin (non modificata nel secondo trasferimento)
        let alice_final = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(alice_final.user_role, Some(UserRole::Admin));

        Ok(())
    }

    /// Test: trasferimento ownership e poi rollback manuale
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_and_rollback(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Trasferisci ownership da Alice a Bob
        repo.transfer_ownership(&1, &2, &1).await?;

        // Verifica il trasferimento
        let alice = repo.read(&(1, 1)).await?.unwrap();
        let bob = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(alice.user_role, Some(UserRole::Admin));
        assert_eq!(bob.user_role, Some(UserRole::Owner));

        // "Rollback" manuale: ritrasferisci ownership da Bob ad Alice
        repo.transfer_ownership(&2, &1, &1).await?;

        // Verifica che siamo tornati allo stato originale (quasi)
        let alice_final = repo.read(&(1, 1)).await?.unwrap();
        let bob_final = repo.read(&(2, 1)).await?.unwrap();

        assert_eq!(alice_final.user_role, Some(UserRole::Owner));
        assert_eq!(bob_final.user_role, Some(UserRole::Admin)); // Bob era MEMBER, ora è ADMIN

        Ok(())
    }

    /// Test: verifica che il trasferimento non modifichi altri campi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_transfer_ownership_preserves_other_fields(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Salva i valori iniziali
        let alice_before = repo.read(&(1, 1)).await?.unwrap();
        let bob_before = repo.read(&(2, 1)).await?.unwrap();

        let alice_member_since = alice_before.member_since;
        let alice_visible_from = alice_before.messages_visible_from;
        let bob_member_since = bob_before.member_since;
        let bob_visible_from = bob_before.messages_visible_from;

        // Trasferisci ownership
        repo.transfer_ownership(&1, &2, &1).await?;

        // Verifica che solo user_role sia cambiato
        let alice_after = repo.read(&(1, 1)).await?.unwrap();
        let bob_after = repo.read(&(2, 1)).await?.unwrap();

        // Verifica che i timestamp non siano cambiati
        assert_eq!(alice_after.member_since, alice_member_since);
        assert_eq!(alice_after.messages_visible_from, alice_visible_from);
        assert_eq!(bob_after.member_since, bob_member_since);
        assert_eq!(bob_after.messages_visible_from, bob_visible_from);

        // Solo i ruoli dovrebbero essere cambiati
        assert_eq!(alice_after.user_role, Some(UserRole::Admin));
        assert_eq!(bob_after.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /*----------------------------------*/
    /* Unit tests: find_many_by_user_id */
    /*----------------------------------*/

    /// Test: trova tutte le chat di un utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Alice (user_id=1) è in 3 chat: General Chat (1), Private Alice-Bob (2), Dev Team (3)
        let result = repo.find_many_by_user_id(&1).await?;

        assert_eq!(result.len(), 3);

        // Verifica che tutti i chat_id siano presenti
        let chat_ids: Vec<i32> = result.iter().map(|m| m.chat_id).collect();
        assert!(chat_ids.contains(&1)); // General Chat
        assert!(chat_ids.contains(&2)); // Private Alice-Bob
        assert!(chat_ids.contains(&3)); // Dev Team

        Ok(())
    }

    /// Test: trova le chat di un utente che è in meno chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_fewer_chats(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Bob (user_id=2) è in 2 chat: General Chat (1), Private Alice-Bob (2)
        let result = repo.find_many_by_user_id(&2).await?;

        assert_eq!(result.len(), 2);

        let chat_ids: Vec<i32> = result.iter().map(|m| m.chat_id).collect();
        assert!(chat_ids.contains(&1)); // General Chat
        assert!(chat_ids.contains(&2)); // Private Alice-Bob
        assert!(!chat_ids.contains(&3)); // NON è in Dev Team

        Ok(())
    }

    /// Test: restituisce lista vuota per utente inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Utente inesistente
        let result = repo.find_many_by_user_id(&999).await?;

        assert_eq!(result.len(), 0);
        assert!(result.is_empty());

        Ok(())
    }

    /// Test: verifica i ruoli dell'utente nelle varie chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_with_roles(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Alice è OWNER in tutte e 3 le sue chat
        let result = repo.find_many_by_user_id(&1).await?;

        for metadata in &result {
            assert_eq!(metadata.user_role, Some(UserRole::Owner));
        }

        // Charlie (user_id=3): MEMBER in General Chat, ADMIN in Dev Team
        let charlie_result = repo.find_many_by_user_id(&3).await?;

        let general_chat = charlie_result.iter().find(|m| m.chat_id == 1).unwrap();
        assert_eq!(general_chat.user_role, Some(UserRole::Member));

        let dev_team = charlie_result.iter().find(|m| m.chat_id == 3).unwrap();
        assert_eq!(dev_team.user_role, Some(UserRole::Admin));

        Ok(())
    }

    /// Test: verifica che i timestamp siano caricati correttamente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_with_timestamps(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let result = repo.find_many_by_user_id(&1).await?;

        // Verifica che tutti i metadata abbiano timestamp validi
        for metadata in result {
            assert!(metadata.member_since > chrono::DateTime::<chrono::Utc>::default());
            assert!(metadata.messages_visible_from > chrono::DateTime::<chrono::Utc>::default());
            assert!(metadata.messages_received_until > chrono::DateTime::<chrono::Utc>::default());
        }

        Ok(())
    }

    /// Test CASCADE: eliminazione di un utente elimina tutti i suoi metadata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_cascade_delete_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Prima: Bob è in 2 chat
        let result_before = repo.find_many_by_user_id(&2).await?;
        assert_eq!(result_before.len(), 2);

        // Elimina Bob
        // CASCADE DELETE eliminerà tutti i suoi metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;

        // Dopo: Bob non dovrebbe avere metadata
        let result_after = repo.find_many_by_user_id(&2).await?;
        assert_eq!(result_after.len(), 0);
        assert!(result_after.is_empty());

        // Verifica nel database
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM userchatmetadata WHERE user_id = ?",
            2
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(
            count.count, 0,
            "Tutti i metadata dell'utente dovrebbero essere eliminati (CASCADE)"
        );

        Ok(())
    }

    /// Test CASCADE: eliminazione di una chat rimuove il metadata per quell'utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_cascade_delete_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Prima: Alice è in 3 chat
        let result_before = repo.find_many_by_user_id(&1).await?;
        assert_eq!(result_before.len(), 3);

        // Elimina General Chat (chat_id=1)
        // CASCADE eliminerà i metadata di tutti gli utenti per quella chat
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Dopo: Alice dovrebbe essere in 2 chat
        let result_after = repo.find_many_by_user_id(&1).await?;
        assert_eq!(result_after.len(), 2);

        // Verifica che la chat eliminata non sia più presente
        let chat_ids: Vec<i32> = result_after.iter().map(|m| m.chat_id).collect();
        assert!(!chat_ids.contains(&1)); // General Chat eliminata
        assert!(chat_ids.contains(&2)); // Private Alice-Bob ancora presente
        assert!(chat_ids.contains(&3)); // Dev Team ancora presente

        Ok(())
    }

    /// Test CASCADE: eliminazione di più chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_cascade_delete_multiple_chats(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Alice è in 3 chat
        let result_before = repo.find_many_by_user_id(&1).await?;
        assert_eq!(result_before.len(), 3);

        // Elimina 2 chat
        sqlx::query!("DELETE FROM chats WHERE chat_id IN (?, ?)", 1, 2)
            .execute(&pool)
            .await?;

        // Alice dovrebbe essere solo in 1 chat
        let result_after = repo.find_many_by_user_id(&1).await?;
        assert_eq!(result_after.len(), 1);
        assert_eq!(result_after[0].chat_id, 3); // Solo Dev Team rimane

        Ok(())
    }

    /// Test CASCADE: eliminazione di utente con molte chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_cascade_delete_user_with_multiple_chats(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Alice (user_id=1) è OWNER in 3 chat
        let result_before = repo.find_many_by_user_id(&1).await?;
        assert_eq!(result_before.len(), 3);

        // Verifica che sia presente in tutte e 3
        let chat_ids_before: Vec<i32> = result_before.iter().map(|m| m.chat_id).collect();
        assert!(chat_ids_before.contains(&1));
        assert!(chat_ids_before.contains(&2));
        assert!(chat_ids_before.contains(&3));

        // Elimina Alice
        // CASCADE eliminerà tutti i suoi metadata in tutte le chat
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 1)
            .execute(&pool)
            .await?;

        // Alice non dovrebbe avere più metadata
        let result_after = repo.find_many_by_user_id(&1).await?;
        assert_eq!(result_after.len(), 0);

        // Le chat dovrebbero esistere ancora (non sono state eliminate)
        let chat1_exists = sqlx::query!("SELECT chat_id FROM chats WHERE chat_id = ?", 1)
            .fetch_optional(&pool)
            .await?;
        let chat2_exists = sqlx::query!("SELECT chat_id FROM chats WHERE chat_id = ?", 2)
            .fetch_optional(&pool)
            .await?;
        let chat3_exists = sqlx::query!("SELECT chat_id FROM chats WHERE chat_id = ?", 3)
            .fetch_optional(&pool)
            .await?;

        assert!(chat1_exists.is_some(), "Chat 1 dovrebbe esistere ancora");
        assert!(chat2_exists.is_some(), "Chat 2 dovrebbe esistere ancora");
        assert!(chat3_exists.is_some(), "Chat 3 dovrebbe esistere ancora");

        Ok(())
    }

    /// Test: aggiunta di un utente a una nuova chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_after_adding_to_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Bob inizialmente è in 2 chat
        let result_before = repo.find_many_by_user_id(&2).await?;
        assert_eq!(result_before.len(), 2);

        // Aggiungi Bob al Dev Team (chat_id=3)
        sqlx::query!(
            r#"
            INSERT INTO userchatmetadata (user_id, chat_id, user_role, member_since, messages_visible_from, messages_received_until)
            VALUES (?, ?, 'MEMBER', NOW(), NOW(), NOW())
            "#,
            2, 3
        )
        .execute(&pool)
        .await?;

        // Ora Bob dovrebbe essere in 3 chat
        let result_after = repo.find_many_by_user_id(&2).await?;
        assert_eq!(result_after.len(), 3);

        let chat_ids: Vec<i32> = result_after.iter().map(|m| m.chat_id).collect();
        assert!(chat_ids.contains(&1));
        assert!(chat_ids.contains(&2));
        assert!(chat_ids.contains(&3)); // Nuovo

        Ok(())
    }

    /// Test: rimozione di un utente da una chat specifica
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_after_leaving_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Alice è in 3 chat
        let result_before = repo.find_many_by_user_id(&1).await?;
        assert_eq!(result_before.len(), 3);

        // Alice lascia General Chat (chat_id=1)
        sqlx::query!(
            "DELETE FROM userchatmetadata WHERE user_id = ? AND chat_id = ?",
            1,
            1
        )
        .execute(&pool)
        .await?;

        // Ora Alice dovrebbe essere in 2 chat
        let result_after = repo.find_many_by_user_id(&1).await?;
        assert_eq!(result_after.len(), 2);

        let chat_ids: Vec<i32> = result_after.iter().map(|m| m.chat_id).collect();
        assert!(!chat_ids.contains(&1)); // Non più in General Chat
        assert!(chat_ids.contains(&2));
        assert!(chat_ids.contains(&3));

        Ok(())
    }

    /// Test: verifica risultati dopo trasferimento ownership
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_after_ownership_transfer(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Stato iniziale
        let alice_before = repo.find_many_by_user_id(&1).await?;
        assert_eq!(alice_before.len(), 3);

        // Verifica che Alice sia OWNER in tutte le sue chat
        for metadata in &alice_before {
            assert_eq!(metadata.user_role, Some(UserRole::Owner));
        }

        // Trasferisci ownership da Alice a Bob nella General Chat
        repo.transfer_ownership(&1, &2, &1).await?;

        // Alice dovrebbe essere ancora in 3 chat
        let alice_after = repo.find_many_by_user_id(&1).await?;
        assert_eq!(alice_after.len(), 3);

        // Ma il suo ruolo in General Chat dovrebbe essere ADMIN
        let general_chat_metadata = alice_after.iter().find(|m| m.chat_id == 1).unwrap();
        assert_eq!(general_chat_metadata.user_role, Some(UserRole::Admin));

        // I suoi ruoli nelle altre chat dovrebbero essere ancora OWNER
        let private_chat_metadata = alice_after.iter().find(|m| m.chat_id == 2).unwrap();
        assert_eq!(private_chat_metadata.user_role, Some(UserRole::Owner));

        let dev_team_metadata = alice_after.iter().find(|m| m.chat_id == 3).unwrap();
        assert_eq!(dev_team_metadata.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test: utente in solo una chat (caso minimo)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_single_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Rimuovi Bob da tutte le chat tranne una
        sqlx::query!(
            "DELETE FROM userchatmetadata WHERE user_id = ? AND chat_id != ?",
            2,
            1
        )
        .execute(&pool)
        .await?;

        // Bob dovrebbe essere solo in 1 chat
        let result = repo.find_many_by_user_id(&2).await?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].chat_id, 1);

        Ok(())
    }

    /// Test CASCADE: eliminazione di tutte le chat di un utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_cascade_delete_all_user_chats(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Bob è in 2 chat (chat_id=1, chat_id=2)
        let result_before = repo.find_many_by_user_id(&2).await?;
        assert_eq!(result_before.len(), 2);

        // Elimina entrambe le chat di Bob
        sqlx::query!("DELETE FROM chats WHERE chat_id IN (?, ?)", 1, 2)
            .execute(&pool)
            .await?;

        // Bob non dovrebbe avere più chat
        let result_after = repo.find_many_by_user_id(&2).await?;
        assert_eq!(result_after.len(), 0);
        assert!(result_after.is_empty());

        // Ma Bob (l'utente) dovrebbe esistere ancora
        let user_exists = sqlx::query!("SELECT user_id FROM users WHERE user_id = ?", 2)
            .fetch_optional(&pool)
            .await?;
        assert!(
            user_exists.is_some(),
            "L'utente Bob dovrebbe esistere ancora"
        );

        Ok(())
    }

    /// Test: verifica ordinamento dei risultati (se presente)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_result_order(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Alice è in 3 chat
        let result = repo.find_many_by_user_id(&1).await?;
        assert_eq!(result.len(), 3);

        // Verifica che tutti i risultati siano validi e abbiano lo stesso user_id
        for metadata in &result {
            assert_eq!(metadata.user_id, 1);
            assert!(metadata.chat_id > 0);
        }

        Ok(())
    }

    /// Test CASCADE: interazione tra eliminazione utente e chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_cascade_mixed_operations(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Stato iniziale: Bob in 2 chat, Charlie in 2 chat
        let bob_before = repo.find_many_by_user_id(&2).await?;
        let charlie_before = repo.find_many_by_user_id(&3).await?;
        assert_eq!(bob_before.len(), 2);
        assert_eq!(charlie_before.len(), 2);

        // Elimina Bob (utente)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;

        // Elimina una chat dove Charlie è membro
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Bob non dovrebbe avere metadata
        let bob_after = repo.find_many_by_user_id(&2).await?;
        assert_eq!(bob_after.len(), 0);

        // Charlie dovrebbe essere in 1 sola chat (Dev Team)
        let charlie_after = repo.find_many_by_user_id(&3).await?;
        assert_eq!(charlie_after.len(), 1);
        assert_eq!(charlie_after[0].chat_id, 3);

        Ok(())
    }

    /*-------------------------*/
    /* Unit tests: create_many */
    /*-------------------------*/

    /// Test: creazione di più metadata in una singola transazione
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            "Test Description",
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Prepara i dati per creare 3 membri contemporaneamente
        let now = chrono::Utc::now();
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: 1,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Owner),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 2,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Admin),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 3,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];

        // Crea tutti i metadata
        let result = repo.create_many(&metadata_list).await?;

        // Verifica che siano stati creati tutti e 3
        assert_eq!(result.len(), 3);

        // Verifica i ruoli
        assert_eq!(result[0].user_role, Some(UserRole::Owner));
        assert_eq!(result[1].user_role, Some(UserRole::Admin));
        assert_eq!(result[2].user_role, Some(UserRole::Member));

        // Verifica che siano stati effettivamente inseriti nel database
        let chat_members = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(chat_members.len(), 3);

        Ok(())
    }

    /// Test: creazione con lista vuota restituisce lista vuota
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_empty_list(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let empty_list: Vec<CreateUserChatMetadataDTO> = vec![];
        let result = repo.create_many(&empty_list).await?;

        assert_eq!(result.len(), 0);
        assert!(result.is_empty());

        Ok(())
    }

    /// Test: creazione di un singolo metadata (caso minimo)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_single_item(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Single Member Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let metadata_list = vec![CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        }];

        let result = repo.create_many(&metadata_list).await?;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].user_id, 1);
        assert_eq!(result[0].chat_id, new_chat_id);

        Ok(())
    }

    /// Test: atomicità della transazione - se uno fallisce, falliscono tutti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_atomicity_user_not_exists(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: 1,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Owner),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 999, // Utente inesistente
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];

        // La creazione dovrebbe fallire
        let result = repo.create_many(&metadata_list).await;
        assert!(result.is_err());

        // Verifica che nessun metadata sia stato creato (rollback automatico)
        let chat_members = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(
            chat_members.len(),
            0,
            "Nessun metadata dovrebbe essere creato (transazione rollback)"
        );

        Ok(())
    }

    /// Test: atomicità - chat inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_atomicity_chat_not_exists(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let now = chrono::Utc::now();
        let metadata_list = vec![CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: 999, // Chat inesistente
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        }];

        // La creazione dovrebbe fallire
        let result = repo.create_many(&metadata_list).await;
        assert!(result.is_err());

        Ok(())
    }

    /// Test: violazione di chiave primaria (user_id, chat_id duplicati)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_duplicate_key_violation(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let now = chrono::Utc::now();
        let metadata_list = vec![CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: 1, // Alice è già nella General Chat
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        }];

        // Dovrebbe fallire perché Alice è già nella chat
        let result = repo.create_many(&metadata_list).await;
        assert!(result.is_err());

        Ok(())
    }

    /// Test CASCADE: creazione e poi eliminazione dell'utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_cascade_delete_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un nuovo utente
        let new_user_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "newuser",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Crea metadata per il nuovo utente
        let now = chrono::Utc::now();
        let metadata_list = vec![CreateUserChatMetadataDTO {
            user_id: new_user_id,
            chat_id: 1,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        }];

        let result = repo.create_many(&metadata_list).await?;
        assert_eq!(result.len(), 1);

        // Verifica che il metadata esista
        let metadata_exists = repo.read(&(new_user_id, 1)).await?;
        assert!(metadata_exists.is_some());

        // Elimina l'utente - CASCADE dovrebbe eliminare il metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", new_user_id)
            .execute(&pool)
            .await?;

        // Verifica che il metadata sia stato eliminato
        let metadata_after = repo.read(&(new_user_id, 1)).await?;
        assert!(
            metadata_after.is_none(),
            "Il metadata dovrebbe essere eliminato (CASCADE)"
        );

        Ok(())
    }

    /// Test CASCADE: creazione e poi eliminazione della chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_cascade_delete_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Temporary Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Aggiungi membri
        let now = chrono::Utc::now();
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: 1,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Owner),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 2,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];

        let result = repo.create_many(&metadata_list).await?;
        assert_eq!(result.len(), 2);

        // Verifica che i metadata esistano
        let members = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(members.len(), 2);

        // Elimina la chat - CASCADE dovrebbe eliminare tutti i metadata
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", new_chat_id)
            .execute(&pool)
            .await?;

        // Verifica che i metadata siano stati eliminati
        let members_after = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(
            members_after.len(),
            0,
            "Tutti i metadata dovrebbero essere eliminati (CASCADE)"
        );

        Ok(())
    }

    /// Test: creazione di molti metadata (stress test)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_large_batch(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea più utenti per il test
        for i in 4..=10 {
            sqlx::query!(
                "INSERT INTO users (user_id, username, password) VALUES (?, ?, ?)",
                i,
                format!("user{}", i),
                "password"
            )
            .execute(&pool)
            .await?;
        }

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Large Group Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Prepara metadata per 10 utenti (user_id 1-10)
        let now = chrono::Utc::now();
        let metadata_list: Vec<CreateUserChatMetadataDTO> = (1..=10)
            .map(|user_id| CreateUserChatMetadataDTO {
                user_id,
                chat_id: new_chat_id,
                user_role: if user_id == 1 {
                    Some(UserRole::Owner)
                } else if user_id <= 3 {
                    Some(UserRole::Admin)
                } else {
                    Some(UserRole::Member)
                },
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            })
            .collect();

        // Crea tutti i metadata
        let result = repo.create_many(&metadata_list).await?;
        assert_eq!(result.len(), 10);

        // Verifica nel database
        let members = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(members.len(), 10);

        Ok(())
    }

    /// Test: verifica ordine di restituzione dei metadata creati
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_preserves_order(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Order Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: 3,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 1,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Owner),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 2,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Admin),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];

        let result = repo.create_many(&metadata_list).await?;

        // Verifica che l'ordine sia preservato
        assert_eq!(result[0].user_id, 3);
        assert_eq!(result[1].user_id, 1);
        assert_eq!(result[2].user_id, 2);

        Ok(())
    }

    /// Test: creazione con diversi ruoli
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_different_roles(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Roles Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: 1,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Owner),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 2,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Admin),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 3,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];

        let result = repo.create_many(&metadata_list).await?;

        // Verifica i ruoli
        assert_eq!(result[0].user_role, Some(UserRole::Owner));
        assert_eq!(result[1].user_role, Some(UserRole::Admin));
        assert_eq!(result[2].user_role, Some(UserRole::Member));

        // Verifica che i ruoli siano stati salvati correttamente
        let owner = repo.read(&(1, new_chat_id)).await?.unwrap();
        let admin = repo.read(&(2, new_chat_id)).await?.unwrap();
        let member = repo.read(&(3, new_chat_id)).await?.unwrap();

        assert_eq!(owner.user_role, Some(UserRole::Owner));
        assert_eq!(admin.user_role, Some(UserRole::Admin));
        assert_eq!(member.user_role, Some(UserRole::Member));

        Ok(())
    }

    /// Test CASCADE: creazione multipla e poi eliminazione di più utenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_cascade_delete_multiple_users(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea nuovi utenti
        let user4_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user4",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let user5_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user5",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Aggiungi i nuovi utenti alla chat
        let now = chrono::Utc::now();
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: user4_id,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Owner),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: user5_id,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];

        repo.create_many(&metadata_list).await?;

        // Verifica creazione
        let members_before = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(members_before.len(), 2);

        // Elimina entrambi gli utenti
        sqlx::query!(
            "DELETE FROM users WHERE user_id IN (?, ?)",
            user4_id,
            user5_id
        )
        .execute(&pool)
        .await?;

        // Verifica che i metadata siano stati eliminati
        let members_after = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(
            members_after.len(),
            0,
            "Tutti i metadata dovrebbero essere eliminati (CASCADE)"
        );

        Ok(())
    }

    /*------------------------------*/
    /* Unit tests: update_user_role */
    /*------------------------------*/

    /// Test: aggiornamento ruolo da MEMBER a ADMIN
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_member_to_admin(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Bob (user_id=2) è MEMBER nella General Chat (chat_id=1)
        let before = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(before.user_role, Some(UserRole::Member));

        // Promuovi Bob ad ADMIN
        let result = repo.update_user_role(&2, &1, &UserRole::Admin).await?;

        assert_eq!(result.user_id, 2);
        assert_eq!(result.chat_id, 1);
        assert_eq!(result.user_role, Some(UserRole::Admin));

        // Verifica nel database
        let after = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(after.user_role, Some(UserRole::Admin));

        Ok(())
    }

    /// Test: aggiornamento ruolo da ADMIN a OWNER
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_admin_to_owner(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Charlie (user_id=3) è ADMIN nel Dev Team (chat_id=3)
        let before = repo.read(&(3, 3)).await?.unwrap();
        assert_eq!(before.user_role, Some(UserRole::Admin));

        // Promuovi Charlie a OWNER
        let result = repo.update_user_role(&3, &3, &UserRole::Owner).await?;

        assert_eq!(result.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test: aggiornamento ruolo da OWNER a MEMBER (demozione)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_owner_to_member(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Alice (user_id=1) è OWNER nella General Chat (chat_id=1)
        let before = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(before.user_role, Some(UserRole::Owner));

        // Degrada Alice a MEMBER
        let result = repo.update_user_role(&1, &1, &UserRole::Member).await?;

        assert_eq!(result.user_role, Some(UserRole::Member));

        // Verifica persistenza
        let after = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(after.user_role, Some(UserRole::Member));

        Ok(())
    }

    /// Test: errore quando user_id non esiste
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_user_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Tentativo di aggiornare utente inesistente
        let result = repo.update_user_role(&999, &1, &UserRole::Admin).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), sqlx::Error::RowNotFound));

        Ok(())
    }

    /// Test: errore quando chat_id non esiste
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_chat_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Tentativo di aggiornare in una chat inesistente
        let result = repo.update_user_role(&1, &999, &UserRole::Admin).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), sqlx::Error::RowNotFound));

        Ok(())
    }

    /// Test: errore quando la combinazione (user_id, chat_id) non esiste
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_metadata_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Bob (user_id=2) non è nel Dev Team (chat_id=3)
        let result = repo.update_user_role(&2, &3, &UserRole::Admin).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), sqlx::Error::RowNotFound));

        Ok(())
    }

    /// Test: verifica che solo user_role cambi, altri campi rimangono invariati
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_preserves_other_fields(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Salva i valori originali
        let before = repo.read(&(2, 1)).await?.unwrap();
        let original_member_since = before.member_since;
        let original_visible_from = before.messages_visible_from;
        let original_received_until = before.messages_received_until;

        // Aggiorna solo il ruolo
        repo.update_user_role(&2, &1, &UserRole::Admin).await?;

        // Verifica che gli altri campi siano invariati
        let after = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(after.member_since, original_member_since);
        assert_eq!(after.messages_visible_from, original_visible_from);
        assert_eq!(after.messages_received_until, original_received_until);
        assert_eq!(after.user_role, Some(UserRole::Admin)); // Solo questo cambia

        Ok(())
    }

    /// Test: aggiornamento ruolo allo stesso valore (idempotenza)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_same_value(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Alice è già OWNER
        let before = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(before.user_role, Some(UserRole::Owner));

        // "Aggiorna" a OWNER (stesso valore)
        let result = repo.update_user_role(&1, &1, &UserRole::Owner).await?;

        assert_eq!(result.user_role, Some(UserRole::Owner));

        // Verifica che funzioni senza problemi
        let after = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(after.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test: aggiornamenti multipli sequenziali
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_multiple_sequential(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Sequenza: MEMBER -> ADMIN -> OWNER -> MEMBER
        let metadata = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(metadata.user_role, Some(UserRole::Member));

        // MEMBER -> ADMIN
        let result1 = repo.update_user_role(&2, &1, &UserRole::Admin).await?;
        assert_eq!(result1.user_role, Some(UserRole::Admin));

        // ADMIN -> OWNER
        let result2 = repo.update_user_role(&2, &1, &UserRole::Owner).await?;
        assert_eq!(result2.user_role, Some(UserRole::Owner));

        // OWNER -> MEMBER
        let result3 = repo.update_user_role(&2, &1, &UserRole::Member).await?;
        assert_eq!(result3.user_role, Some(UserRole::Member));

        // Verifica finale
        let final_state = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(final_state.user_role, Some(UserRole::Member));

        Ok(())
    }

    /// Test: aggiornamento in chat privata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_private_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Private Alice-Bob (chat_id=2): Bob è MEMBER
        let before = repo.read(&(2, 2)).await?.unwrap();
        assert_eq!(before.user_role, Some(UserRole::Member));

        // Promuovi Bob a OWNER nella chat privata
        let result = repo.update_user_role(&2, &2, &UserRole::Owner).await?;

        assert_eq!(result.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test CASCADE: aggiornamento ruolo e poi eliminazione utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_cascade_delete_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Promuovi Bob ad ADMIN
        repo.update_user_role(&2, &1, &UserRole::Admin).await?;

        // Verifica l'aggiornamento
        let after_update = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(after_update.user_role, Some(UserRole::Admin));

        // Elimina Bob - CASCADE dovrebbe eliminare tutti i suoi metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;

        // Il metadata non dovrebbe più esistere
        let after_delete = repo.read(&(2, 1)).await?;
        assert!(
            after_delete.is_none(),
            "Il metadata dovrebbe essere eliminato (CASCADE)"
        );

        Ok(())
    }

    /// Test CASCADE: aggiornamento ruolo e poi eliminazione chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_cascade_delete_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Promuovi Bob ad ADMIN nella General Chat
        repo.update_user_role(&2, &1, &UserRole::Admin).await?;

        // Verifica l'aggiornamento
        let after_update = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(after_update.user_role, Some(UserRole::Admin));

        // Elimina la chat - CASCADE dovrebbe eliminare tutti i metadata
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Nessun metadata dovrebbe esistere per questa chat
        let members = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(
            members.len(),
            0,
            "Tutti i metadata dovrebbero essere eliminati (CASCADE)"
        );

        Ok(())
    }

    /// Test: aggiornamenti concorrenti su utenti diversi nella stessa chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_multiple_users_same_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // General Chat (chat_id=1): aggiorna ruoli di più utenti
        repo.update_user_role(&2, &1, &UserRole::Admin).await?;
        repo.update_user_role(&3, &1, &UserRole::Admin).await?;

        // Verifica che entrambi siano stati aggiornati
        let bob = repo.read(&(2, 1)).await?.unwrap();
        let charlie = repo.read(&(3, 1)).await?.unwrap();

        assert_eq!(bob.user_role, Some(UserRole::Admin));
        assert_eq!(charlie.user_role, Some(UserRole::Admin));

        // Alice dovrebbe essere ancora OWNER (non modificata)
        let alice = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(alice.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test: aggiornamento ruolo dopo transfer_ownership
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_after_transfer_ownership(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Trasferisci ownership da Alice a Bob
        repo.transfer_ownership(&1, &2, &1).await?;

        // Alice dovrebbe essere ADMIN ora
        let alice = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(alice.user_role, Some(UserRole::Admin));

        // Degrada Alice a MEMBER
        repo.update_user_role(&1, &1, &UserRole::Member).await?;

        let alice_after = repo.read(&(1, 1)).await?.unwrap();
        assert_eq!(alice_after.user_role, Some(UserRole::Member));

        // Bob dovrebbe essere ancora OWNER
        let bob = repo.read(&(2, 1)).await?.unwrap();
        assert_eq!(bob.user_role, Some(UserRole::Owner));

        Ok(())
    }

    /// Test: verifica che update_user_role funzioni con tutti e tre i ruoli
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_all_roles(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat con 3 membri
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: 1,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 2,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
            CreateUserChatMetadataDTO {
                user_id: 3,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];

        repo.create_many(&metadata_list).await?;

        // Aggiorna a ruoli diversi
        repo.update_user_role(&1, &new_chat_id, &UserRole::Owner)
            .await?;
        repo.update_user_role(&2, &new_chat_id, &UserRole::Admin)
            .await?;
        repo.update_user_role(&3, &new_chat_id, &UserRole::Member)
            .await?; // Rimane MEMBER

        // Verifica
        let user1 = repo.read(&(1, new_chat_id)).await?.unwrap();
        let user2 = repo.read(&(2, new_chat_id)).await?.unwrap();
        let user3 = repo.read(&(3, new_chat_id)).await?.unwrap();

        assert_eq!(user1.user_role, Some(UserRole::Owner));
        assert_eq!(user2.user_role, Some(UserRole::Admin));
        assert_eq!(user3.user_role, Some(UserRole::Member));

        Ok(())
    }

    /// Test CASCADE: aggiornamento di più utenti e poi eliminazione della chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_cascade_multiple_updates_then_delete(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Aggiorna più ruoli nella General Chat
        repo.update_user_role(&2, &1, &UserRole::Admin).await?;
        repo.update_user_role(&3, &1, &UserRole::Admin).await?;

        // Verifica gli aggiornamenti
        let members_before = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(members_before.len(), 3);

        // Conta gli admin
        let admin_count = members_before
            .iter()
            .filter(|m| m.user_role == Some(UserRole::Admin))
            .count();
        assert_eq!(admin_count, 2);

        // Elimina la chat
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Tutti i metadata dovrebbero essere eliminati
        let members_after = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(members_after.len(), 0);

        Ok(())
    }

    /// Test: verifica che rows_affected sia corretto
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_user_role_rows_affected(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Aggiornamento valido
        let result = repo.update_user_role(&2, &1, &UserRole::Admin).await;
        assert!(result.is_ok(), "L'aggiornamento dovrebbe avere successo");

        // Aggiornamento invalido (metadata inesistente)
        let result_invalid = repo.update_user_role(&999, &999, &UserRole::Admin).await;
        assert!(result_invalid.is_err(), "Dovrebbe fallire con RowNotFound");

        Ok(())
    }

    /*------------------------------------*/
    /* Unit tests: create (casi negativi) */
    /*------------------------------------*/

    /// Test NEGATIVO: errore con user_id inesistente (violazione foreign key)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_error_user_not_exists(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 999, // Utente inesistente
            chat_id: 1,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        let result = repo.create(&dto).await;

        assert!(
            result.is_err(),
            "Dovrebbe fallire con foreign key violation su user_id"
        );

        Ok(())
    }

    /// Test NEGATIVO: errore con chat_id inesistente (violazione foreign key)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_error_chat_not_exists(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: 999, // Chat inesistente
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        let result = repo.create(&dto).await;

        assert!(
            result.is_err(),
            "Dovrebbe fallire con foreign key violation su chat_id"
        );

        Ok(())
    }

    /// Test NEGATIVO: errore con entrambi user_id e chat_id inesistenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_error_both_not_exist(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 888,
            chat_id: 999,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        let result = repo.create(&dto).await;

        assert!(
            result.is_err(),
            "Dovrebbe fallire con foreign key violation"
        );

        Ok(())
    }

    /// Test NEGATIVO: errore con chiave primaria duplicata (user_id, chat_id)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_error_duplicate_key(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 1, // Alice è già nella General Chat (fixtures)
            chat_id: 1,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        let result = repo.create(&dto).await;

        assert!(
            result.is_err(),
            "Dovrebbe fallire con duplicate primary key error"
        );

        Ok(())
    }

    /// Test NEGATIVO: errore tentando di creare dopo che l'utente esiste ma viene eliminato durante l'operazione
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_error_user_deleted_before_insert(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un nuovo utente
        let new_user_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "tempuser",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Elimina immediatamente l'utente
        sqlx::query!("DELETE FROM users WHERE user_id = ?", new_user_id)
            .execute(&pool)
            .await?;

        // Prova a creare il metadata per l'utente appena eliminato
        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: new_user_id,
            chat_id: 1,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        let result = repo.create(&dto).await;

        assert!(
            result.is_err(),
            "Dovrebbe fallire perché l'utente è stato eliminato"
        );

        Ok(())
    }

    /// Test NEGATIVO: errore tentando di creare dopo che la chat viene eliminata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_error_chat_deleted_before_insert(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Temporary Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Elimina immediatamente la chat
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", new_chat_id)
            .execute(&pool)
            .await?;

        // Prova a creare il metadata per la chat appena eliminata
        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        let result = repo.create(&dto).await;

        assert!(
            result.is_err(),
            "Dovrebbe fallire perché la chat è stata eliminata"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: creazione e poi eliminazione dell'utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_cascade_delete_user_removes_metadata(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un nuovo utente
        let new_user_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "testuser",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Crea metadata per il nuovo utente
        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: new_user_id,
            chat_id: 1,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&dto).await?;

        // Verifica che il metadata esista
        let exists = repo.read(&(new_user_id, 1)).await?;
        assert!(
            exists.is_some(),
            "Il metadata dovrebbe esistere dopo la creazione"
        );

        // Elimina l'utente - CASCADE dovrebbe eliminare il metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", new_user_id)
            .execute(&pool)
            .await?;

        // Verifica che il metadata sia stato eliminato
        let after_delete = repo.read(&(new_user_id, 1)).await?;
        assert!(
            after_delete.is_none(),
            "Il metadata dovrebbe essere eliminato automaticamente (CASCADE DELETE su user_id)"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: creazione e poi eliminazione della chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_cascade_delete_chat_removes_metadata(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Temporary Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Crea metadata
        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&dto).await?;

        // Verifica che il metadata esista
        let exists = repo.read(&(1, new_chat_id)).await?;
        assert!(
            exists.is_some(),
            "Il metadata dovrebbe esistere dopo la creazione"
        );

        // Elimina la chat - CASCADE dovrebbe eliminare il metadata
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", new_chat_id)
            .execute(&pool)
            .await?;

        // Verifica che il metadata sia stato eliminato
        let after_delete = repo.read(&(1, new_chat_id)).await?;
        assert!(
            after_delete.is_none(),
            "Il metadata dovrebbe essere eliminato automaticamente (CASCADE DELETE su chat_id)"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: creazione di più metadata per utenti diversi nella stessa chat, poi eliminazione della chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_cascade_delete_chat_removes_all_members_metadata(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Group",
            Some("Group to be deleted"),
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();

        // Crea metadata per 3 utenti
        for user_id in [1, 2, 3] {
            let dto = CreateUserChatMetadataDTO {
                user_id,
                chat_id: new_chat_id,
                user_role: Some(if user_id == 1 {
                    UserRole::Owner
                } else {
                    UserRole::Member
                }),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            };
            repo.create(&dto).await?;
        }

        // Verifica che tutti i metadata esistano
        let members = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(members.len(), 3, "Dovrebbero esserci 3 membri");

        // Elimina la chat
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", new_chat_id)
            .execute(&pool)
            .await?;

        // Verifica che TUTTI i metadata siano stati eliminati (CASCADE)
        let members_after = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(
            members_after.len(),
            0,
            "Tutti i metadata dovrebbero essere eliminati (CASCADE DELETE)"
        );

        // Verifica anche singolarmente
        for user_id in [1, 2, 3] {
            let metadata = repo.read(&(user_id, new_chat_id)).await?;
            assert!(
                metadata.is_none(),
                "Il metadata per user_id {} dovrebbe essere eliminato",
                user_id
            );
        }

        Ok(())
    }

    /// Test NEGATIVO CASCADE: creazione di metadata per lo stesso utente in più chat, poi eliminazione dell'utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_cascade_delete_user_removes_all_chats_metadata(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un nuovo utente
        let new_user_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "testuser",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Crea 2 nuove chat
        let chat_ids: Vec<i32> = vec![
            sqlx::query!(
                "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
                "Test Chat 1",
                None::<String>,
                "GROUP"
            )
            .execute(&pool)
            .await?
            .last_insert_id() as i32,
            sqlx::query!(
                "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
                "Test Chat 2",
                None::<String>,
                "GROUP"
            )
            .execute(&pool)
            .await?
            .last_insert_id() as i32,
        ];

        let now = chrono::Utc::now();

        // Aggiungi l'utente a entrambe le chat
        for &chat_id in &chat_ids {
            let dto = CreateUserChatMetadataDTO {
                user_id: new_user_id,
                chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            };
            repo.create(&dto).await?;
        }

        // Verifica che l'utente sia in 2 chat
        let user_chats = repo.find_many_by_user_id(&new_user_id).await?;
        assert_eq!(user_chats.len(), 2, "L'utente dovrebbe essere in 2 chat");

        // Elimina l'utente
        sqlx::query!("DELETE FROM users WHERE user_id = ?", new_user_id)
            .execute(&pool)
            .await?;

        // Verifica che TUTTI i metadata siano stati eliminati (CASCADE)
        let user_chats_after = repo.find_many_by_user_id(&new_user_id).await?;
        assert_eq!(
            user_chats_after.len(),
            0,
            "Tutti i metadata dovrebbero essere eliminati (CASCADE DELETE)"
        );

        // Verifica anche singolarmente
        for &chat_id in &chat_ids {
            let metadata = repo.read(&(new_user_id, chat_id)).await?;
            assert!(
                metadata.is_none(),
                "Il metadata per chat_id {} dovrebbe essere eliminato",
                chat_id
            );
        }

        Ok(())
    }

    /// Test NEGATIVO CASCADE: scenario complesso con creazioni multiple e CASCADE su entrambe le FK
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_cascade_complex_scenario(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea 2 nuovi utenti
        let user1_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user1",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let user2_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user2",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Crea 2 nuove chat
        let chat1_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Chat 1",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let chat2_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Chat 2",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();

        // Crea una matrice di metadata: ogni utente in ogni chat
        for &user_id in &[user1_id, user2_id] {
            for &chat_id in &[chat1_id, chat2_id] {
                let dto = CreateUserChatMetadataDTO {
                    user_id,
                    chat_id,
                    user_role: Some(UserRole::Member),
                    member_since: now,
                    messages_visible_from: now,
                    messages_received_until: now,
                };
                repo.create(&dto).await?;
            }
        }

        // Verifica: 4 metadata creati (2 utenti x 2 chat)
        assert_eq!(repo.find_many_by_chat_id(&chat1_id).await?.len(), 2);
        assert_eq!(repo.find_many_by_chat_id(&chat2_id).await?.len(), 2);
        assert_eq!(repo.find_many_by_user_id(&user1_id).await?.len(), 2);
        assert_eq!(repo.find_many_by_user_id(&user2_id).await?.len(), 2);

        // Elimina user1 - dovrebbe rimuovere 2 metadata (user1 in chat1 e chat2)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", user1_id)
            .execute(&pool)
            .await?;

        assert_eq!(
            repo.find_many_by_chat_id(&chat1_id).await?.len(),
            1,
            "Chat1 dovrebbe avere 1 membro"
        );
        assert_eq!(
            repo.find_many_by_chat_id(&chat2_id).await?.len(),
            1,
            "Chat2 dovrebbe avere 1 membro"
        );
        assert_eq!(
            repo.find_many_by_user_id(&user1_id).await?.len(),
            0,
            "User1 non dovrebbe avere metadata"
        );

        // Elimina chat1 - dovrebbe rimuovere 1 metadata (user2 in chat1)
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", chat1_id)
            .execute(&pool)
            .await?;

        assert_eq!(
            repo.find_many_by_chat_id(&chat1_id).await?.len(),
            0,
            "Chat1 non dovrebbe avere membri"
        );
        assert_eq!(
            repo.find_many_by_chat_id(&chat2_id).await?.len(),
            1,
            "Chat2 dovrebbe ancora avere 1 membro"
        );
        assert_eq!(
            repo.find_many_by_user_id(&user2_id).await?.len(),
            1,
            "User2 dovrebbe essere in 1 chat"
        );

        // Verifica che rimanga solo il metadata di user2 in chat2
        let remaining = repo.read(&(user2_id, chat2_id)).await?;
        assert!(remaining.is_some(), "Dovrebbe rimanere solo user2 in chat2");

        Ok(())
    }

    /// Test NEGATIVO: tentativo di creare dopo che un altro metadata nella stessa chat causa un errore
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_error_isolation_between_creates(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();

        // Prima creazione: successo
        let dto1 = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };
        let result1 = repo.create(&dto1).await;
        assert!(
            result1.is_ok(),
            "La prima creazione dovrebbe avere successo"
        );

        // Seconda creazione con user_id invalido: fallimento
        let dto2 = CreateUserChatMetadataDTO {
            user_id: 999,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };
        let result2 = repo.create(&dto2).await;
        assert!(result2.is_err(), "La seconda creazione dovrebbe fallire");

        // Terza creazione valida: dovrebbe avere successo nonostante il fallimento precedente
        let dto3 = CreateUserChatMetadataDTO {
            user_id: 2,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };
        let result3 = repo.create(&dto3).await;
        assert!(
            result3.is_ok(),
            "La terza creazione dovrebbe avere successo (isolamento degli errori)"
        );

        // Verifica: dovrebbero esserci solo 2 metadata (quello fallito non è stato inserito)
        let members = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(
            members.len(),
            2,
            "Dovrebbero esserci solo 2 membri (quello fallito non è stato inserito)"
        );

        Ok(())
    }

    /*----------------------------------*/
    /* Unit tests: read (casi negativi) */
    /*----------------------------------*/

    /// Test NEGATIVO: read di metadata inesistente (user_id valido ma non in quella chat)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_not_exists_valid_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Bob (user_id=2) non è nella Dev Team chat (chat_id=3)
        let result = repo.read(&(2, 3)).await?;

        assert!(result.is_none(), "Il metadata non dovrebbe esistere");

        Ok(())
    }

    /// Test NEGATIVO: read di metadata inesistente (chat_id valido ma utente non membro)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_not_exists_valid_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Charlie (user_id=3) non è nella chat privata Alice-Bob (chat_id=2)
        let result = repo.read(&(3, 2)).await?;

        assert!(result.is_none(), "Il metadata non dovrebbe esistere");

        Ok(())
    }

    /// Test NEGATIVO: read con user_id completamente inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_invalid_user_id(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let result = repo.read(&(999, 1)).await?;

        assert!(
            result.is_none(),
            "Nessun metadata dovrebbe esistere per user_id inesistente"
        );

        Ok(())
    }

    /// Test NEGATIVO: read con chat_id completamente inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_invalid_chat_id(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let result = repo.read(&(1, 999)).await?;

        assert!(
            result.is_none(),
            "Nessun metadata dovrebbe esistere per chat_id inesistente"
        );

        Ok(())
    }

    /// Test NEGATIVO: read con entrambi user_id e chat_id inesistenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_both_invalid(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let result = repo.read(&(888, 999)).await?;

        assert!(result.is_none(), "Nessun metadata dovrebbe esistere");

        Ok(())
    }

    /// Test NEGATIVO: read con ID negativi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_negative_ids(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let result = repo.read(&(-1, -1)).await?;

        assert!(
            result.is_none(),
            "Nessun metadata dovrebbe esistere per ID negativi"
        );

        Ok(())
    }

    /// Test NEGATIVO: read con ID zero
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_zero_ids(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let result = repo.read(&(0, 0)).await?;

        assert!(
            result.is_none(),
            "Nessun metadata dovrebbe esistere per ID zero"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: read dopo delete dell'utente (CASCADE DELETE)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_after_cascade_delete_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Bob (user_id=2) è nella General Chat (chat_id=1)
        let before = repo.read(&(2, 1)).await?;
        assert!(before.is_some(), "Bob dovrebbe essere nella General Chat");

        // Elimina Bob - CASCADE dovrebbe eliminare il metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;

        // Read dovrebbe restituire None
        let after = repo.read(&(2, 1)).await?;
        assert!(
            after.is_none(),
            "Il metadata dovrebbe essere eliminato (CASCADE DELETE su user_id)"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: read dopo delete della chat (CASCADE DELETE)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_after_cascade_delete_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Alice (user_id=1) è nella General Chat (chat_id=1)
        let before = repo.read(&(1, 1)).await?;
        assert!(before.is_some(), "Alice dovrebbe essere nella General Chat");

        // Elimina la General Chat - CASCADE dovrebbe eliminare tutti i metadata
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Read dovrebbe restituire None
        let after = repo.read(&(1, 1)).await?;
        assert!(
            after.is_none(),
            "Il metadata dovrebbe essere eliminato (CASCADE DELETE su chat_id)"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: read multipli dopo delete dell'utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_multiple_after_cascade_delete_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Alice è in 3 chat (fixtures)
        let chats_before = repo.find_many_by_user_id(&1).await?;
        let alice_chat_ids: Vec<i32> = chats_before.iter().map(|m| m.chat_id).collect();

        // Verifica che Alice sia in tutte quelle chat
        for &chat_id in &alice_chat_ids {
            let metadata = repo.read(&(1, chat_id)).await?;
            assert!(
                metadata.is_some(),
                "Alice dovrebbe essere nella chat {}",
                chat_id
            );
        }

        // Elimina Alice
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 1)
            .execute(&pool)
            .await?;

        // Tutte le read dovrebbero restituire None
        for &chat_id in &alice_chat_ids {
            let metadata = repo.read(&(1, chat_id)).await?;
            assert!(
                metadata.is_none(),
                "Il metadata di Alice nella chat {} dovrebbe essere eliminato",
                chat_id
            );
        }

        Ok(())
    }

    /// Test NEGATIVO CASCADE: read multipli dopo delete della chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_multiple_after_cascade_delete_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // General Chat ha 3 membri
        let members_before = repo.find_many_by_chat_id(&1).await?;
        let user_ids: Vec<i32> = members_before.iter().map(|m| m.user_id).collect();

        assert_eq!(user_ids.len(), 3, "La General Chat dovrebbe avere 3 membri");

        // Verifica che tutti i membri siano nella chat
        for &user_id in &user_ids {
            let metadata = repo.read(&(user_id, 1)).await?;
            assert!(
                metadata.is_some(),
                "L'utente {} dovrebbe essere nella General Chat",
                user_id
            );
        }

        // Elimina la General Chat
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Tutte le read dovrebbero restituire None
        for &user_id in &user_ids {
            let metadata = repo.read(&(user_id, 1)).await?;
            assert!(
                metadata.is_none(),
                "Il metadata dell'utente {} dovrebbe essere eliminato",
                user_id
            );
        }

        Ok(())
    }

    /// Test NEGATIVO: read dopo delete manuale (non CASCADE)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_after_manual_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un nuovo metadata
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&dto).await?;

        // Verifica che esista
        let before = repo.read(&(1, new_chat_id)).await?;
        assert!(before.is_some(), "Il metadata dovrebbe esistere");

        // Delete manuale
        repo.delete(&(1, new_chat_id)).await?;

        // Read dovrebbe restituire None
        let after = repo.read(&(1, new_chat_id)).await?;
        assert!(
            after.is_none(),
            "Il metadata non dovrebbe più esistere dopo delete"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: read dopo creazione e immediata eliminazione CASCADE
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_after_create_and_immediate_cascade_delete(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un nuovo utente
        let new_user_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "tempuser",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Crea metadata
        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: new_user_id,
            chat_id: 1,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&dto).await?;

        // Verifica che esista
        let before = repo.read(&(new_user_id, 1)).await?;
        assert!(before.is_some(), "Il metadata dovrebbe esistere");

        // Elimina l'utente immediatamente
        sqlx::query!("DELETE FROM users WHERE user_id = ?", new_user_id)
            .execute(&pool)
            .await?;

        // Read dovrebbe restituire None
        let after = repo.read(&(new_user_id, 1)).await?;
        assert!(
            after.is_none(),
            "Il metadata dovrebbe essere eliminato immediatamente (CASCADE)"
        );

        Ok(())
    }

    /// Test NEGATIVO: read con combinazioni di ID validi e invalidi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_mixed_valid_invalid_ids(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // User valido, chat invalida
        let result1 = repo.read(&(1, 999)).await?;
        assert!(result1.is_none());

        // User invalido, chat valida
        let result2 = repo.read(&(999, 1)).await?;
        assert!(result2.is_none());

        // User negativo, chat valida
        let result3 = repo.read(&(-5, 1)).await?;
        assert!(result3.is_none());

        // User valido, chat negativa
        let result4 = repo.read(&(1, -5)).await?;
        assert!(result4.is_none());

        Ok(())
    }

    /// Test NEGATIVO CASCADE: scenario complesso - read dopo eliminazioni multiple
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_cascade_complex_scenario(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una matrice di test: 2 utenti, 2 chat
        let user1_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user1",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let user2_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user2",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let chat1_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Chat 1",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let chat2_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Chat 2",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();

        // Crea 4 metadata (user1 in chat1, user1 in chat2, user2 in chat1, user2 in chat2)
        for &user_id in &[user1_id, user2_id] {
            for &chat_id in &[chat1_id, chat2_id] {
                let dto = CreateUserChatMetadataDTO {
                    user_id,
                    chat_id,
                    user_role: Some(UserRole::Member),
                    member_since: now,
                    messages_visible_from: now,
                    messages_received_until: now,
                };
                repo.create(&dto).await?;
            }
        }

        // Verifica che tutti esistano
        assert!(repo.read(&(user1_id, chat1_id)).await?.is_some());
        assert!(repo.read(&(user1_id, chat2_id)).await?.is_some());
        assert!(repo.read(&(user2_id, chat1_id)).await?.is_some());
        assert!(repo.read(&(user2_id, chat2_id)).await?.is_some());

        // Elimina user1 - dovrebbe eliminare 2 metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", user1_id)
            .execute(&pool)
            .await?;

        assert!(
            repo.read(&(user1_id, chat1_id)).await?.is_none(),
            "user1-chat1 dovrebbe essere eliminato"
        );
        assert!(
            repo.read(&(user1_id, chat2_id)).await?.is_none(),
            "user1-chat2 dovrebbe essere eliminato"
        );
        assert!(
            repo.read(&(user2_id, chat1_id)).await?.is_some(),
            "user2-chat1 dovrebbe esistere"
        );
        assert!(
            repo.read(&(user2_id, chat2_id)).await?.is_some(),
            "user2-chat2 dovrebbe esistere"
        );

        // Elimina chat1 - dovrebbe eliminare 1 metadata rimanente
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", chat1_id)
            .execute(&pool)
            .await?;

        assert!(
            repo.read(&(user2_id, chat1_id)).await?.is_none(),
            "user2-chat1 dovrebbe essere eliminato"
        );
        assert!(
            repo.read(&(user2_id, chat2_id)).await?.is_some(),
            "user2-chat2 dovrebbe ancora esistere"
        );

        Ok(())
    }

    /// Test NEGATIVO: read dopo update fallito non dovrebbe influenzare il risultato
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_after_failed_operations(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Tenta di creare con FK invalida (dovrebbe fallire)
        let now = chrono::Utc::now();
        let invalid_dto = CreateUserChatMetadataDTO {
            user_id: 999,
            chat_id: 1,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        let create_result = repo.create(&invalid_dto).await;
        assert!(create_result.is_err(), "La creazione dovrebbe fallire");

        // Read dovrebbe confermare che non esiste
        let after_failed_create = repo.read(&(999, 1)).await?;
        assert!(
            after_failed_create.is_none(),
            "Non dovrebbe esistere dopo creazione fallita"
        );

        // Tenta di leggere un metadata esistente per confermare che il database è ancora consistente
        let existing = repo.read(&(1, 1)).await?;
        assert!(
            existing.is_some(),
            "I metadata esistenti dovrebbero essere ancora accessibili"
        );

        Ok(())
    }

    /// Test NEGATIVO: read ripetuti dopo CASCADE DELETE confermano persistenza dell'eliminazione
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_multiple_times_after_cascade(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Bob è nella General Chat
        assert!(repo.read(&(2, 1)).await?.is_some());

        // Elimina Bob
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;

        // Leggi multiple volte - dovrebbe sempre restituire None
        for _ in 0..5 {
            let result = repo.read(&(2, 1)).await?;
            assert!(
                result.is_none(),
                "Dovrebbe sempre restituire None dopo CASCADE DELETE"
            );
        }

        Ok(())
    }

    /*------------------------------------*/
    /* Unit tests: update (casi negativi) */
    /*------------------------------------*/

    /// Test NEGATIVO: update di metadata inesistente (user_id e chat_id non associati)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_not_exists(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Bob (user_id=2) non è nella Dev Team (chat_id=3)
        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(2, 3), &update_dto).await;

        assert!(
            result.is_err(),
            "L'update dovrebbe fallire per metadata inesistente"
        );

        Ok(())
    }

    /// Test NEGATIVO: update con user_id completamente inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_invalid_user_id(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Member),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(999, 1), &update_dto).await;

        assert!(
            result.is_err(),
            "L'update dovrebbe fallire per user_id inesistente"
        );

        Ok(())
    }

    /// Test NEGATIVO: update con chat_id completamente inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_invalid_chat_id(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Owner),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(1, 999), &update_dto).await;

        assert!(
            result.is_err(),
            "L'update dovrebbe fallire per chat_id inesistente"
        );

        Ok(())
    }

    /// Test NEGATIVO: update con entrambi user_id e chat_id inesistenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_both_invalid(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(888, 999), &update_dto).await;

        assert!(
            result.is_err(),
            "L'update dovrebbe fallire per entrambi gli ID inesistenti"
        );

        Ok(())
    }

    /// Test NEGATIVO: update con ID negativi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_negative_ids(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Member),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(-1, -1), &update_dto).await;

        assert!(result.is_err(), "L'update dovrebbe fallire per ID negativi");

        Ok(())
    }

    /// Test NEGATIVO: update con ID zero
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_zero_ids(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Owner),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(0, 0), &update_dto).await;

        assert!(result.is_err(), "L'update dovrebbe fallire per ID zero");

        Ok(())
    }

    /// Test NEGATIVO CASCADE: update e poi eliminazione dell'utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_then_cascade_delete_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un nuovo utente e metadata
        let new_user_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "testuser",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: new_user_id,
            chat_id: 1,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&dto).await?;

        // Update del metadata
        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let updated = repo.update(&(new_user_id, 1), &update_dto).await?;
        assert_eq!(updated.user_role, Some(UserRole::Admin));

        // Elimina l'utente - CASCADE dovrebbe eliminare il metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", new_user_id)
            .execute(&pool)
            .await?;

        // Verifica che il metadata sia stato eliminato
        let after_delete = repo.read(&(new_user_id, 1)).await?;
        assert!(
            after_delete.is_none(),
            "Il metadata dovrebbe essere eliminato (CASCADE DELETE)"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: update e poi eliminazione della chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_then_cascade_delete_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        // Crea metadata
        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&dto).await?;

        // Update del metadata
        let future = chrono::Utc::now() + chrono::Duration::days(30);
        let update_dto = UpdateUserChatMetadataDTO {
            user_role: None,
            messages_visible_from: None,
            messages_received_until: Some(future),
        };

        let updated = repo.update(&(1, new_chat_id), &update_dto).await?;
        // Verifica che l'update sia avvenuto (il timestamp dovrebbe essere vicino a 'future')
        assert!(
            updated.messages_received_until > now,
            "Il timestamp dovrebbe essere stato aggiornato"
        );

        // Elimina la chat - CASCADE dovrebbe eliminare il metadata
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", new_chat_id)
            .execute(&pool)
            .await?;

        // Verifica che il metadata sia stato eliminato
        let after_delete = repo.read(&(1, new_chat_id)).await?;
        assert!(
            after_delete.is_none(),
            "Il metadata dovrebbe essere eliminato (CASCADE DELETE)"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: update multipli e poi eliminazione CASCADE
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_multiple_then_cascade_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Update più utenti nella General Chat
        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        repo.update(&(2, 1), &update_dto).await?;
        repo.update(&(3, 1), &update_dto).await?;

        // Verifica gli update
        let bob = repo.read(&(2, 1)).await?.unwrap();
        let charlie = repo.read(&(3, 1)).await?.unwrap();
        assert_eq!(bob.user_role, Some(UserRole::Admin));
        assert_eq!(charlie.user_role, Some(UserRole::Admin));

        // Elimina la chat
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Tutti i metadata dovrebbero essere eliminati
        assert!(repo.read(&(1, 1)).await?.is_none());
        assert!(repo.read(&(2, 1)).await?.is_none());
        assert!(repo.read(&(3, 1)).await?.is_none());

        Ok(())
    }

    /// Test NEGATIVO: tentativo di update dopo delete dell'utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_after_user_deleted(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Bob è nella General Chat
        let before = repo.read(&(2, 1)).await?;
        assert!(before.is_some());

        // Elimina Bob - CASCADE elimina il metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;

        // Tentativo di update dovrebbe fallire
        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(2, 1), &update_dto).await;

        assert!(
            result.is_err(),
            "L'update dovrebbe fallire perché il metadata è stato eliminato (CASCADE)"
        );

        Ok(())
    }

    /// Test NEGATIVO: tentativo di update dopo delete della chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_after_chat_deleted(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Alice è nella General Chat
        let before = repo.read(&(1, 1)).await?;
        assert!(before.is_some());

        // Elimina la General Chat - CASCADE elimina tutti i metadata
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Tentativo di update dovrebbe fallire
        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(1, 1), &update_dto).await;

        assert!(
            result.is_err(),
            "L'update dovrebbe fallire perché il metadata è stato eliminato (CASCADE)"
        );

        Ok(())
    }

    /// Test NEGATIVO: update dopo delete manuale del metadata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_after_manual_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un metadata
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&dto).await?;

        // Delete manuale
        repo.delete(&(1, new_chat_id)).await?;

        // Tentativo di update dovrebbe fallire
        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(1, new_chat_id), &update_dto).await;

        assert!(
            result.is_err(),
            "L'update dovrebbe fallire perché il metadata è stato eliminato"
        );

        Ok(())
    }

    /// Test NEGATIVO: update con DTO vuoto (nessun campo da aggiornare) su metadata inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_empty_dto_not_exists(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // DTO vuoto
        let update_dto = UpdateUserChatMetadataDTO {
            user_role: None,
            messages_visible_from: None,
            messages_received_until: None,
        };

        // Anche se il DTO è vuoto, dovrebbe fallire perché il metadata non esiste
        let result = repo.update(&(999, 999), &update_dto).await;

        assert!(
            result.is_err(),
            "L'update dovrebbe fallire per metadata inesistente, anche con DTO vuoto"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: scenario complesso con update e eliminazioni multiple
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_cascade_complex_scenario(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea 2 utenti e 2 chat
        let user1_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user1",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let user2_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user2",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let chat1_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Chat 1",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let chat2_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Chat 2",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();

        // Crea 4 metadata
        for &user_id in &[user1_id, user2_id] {
            for &chat_id in &[chat1_id, chat2_id] {
                let dto = CreateUserChatMetadataDTO {
                    user_id,
                    chat_id,
                    user_role: Some(UserRole::Member),
                    member_since: now,
                    messages_visible_from: now,
                    messages_received_until: now,
                };
                repo.create(&dto).await?;
            }
        }

        // Update tutti a Admin
        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        for &user_id in &[user1_id, user2_id] {
            for &chat_id in &[chat1_id, chat2_id] {
                repo.update(&(user_id, chat_id), &update_dto).await?;
            }
        }

        // Verifica che tutti siano Admin
        for &user_id in &[user1_id, user2_id] {
            for &chat_id in &[chat1_id, chat2_id] {
                let metadata = repo.read(&(user_id, chat_id)).await?.unwrap();
                assert_eq!(metadata.user_role, Some(UserRole::Admin));
            }
        }

        // Elimina user1
        sqlx::query!("DELETE FROM users WHERE user_id = ?", user1_id)
            .execute(&pool)
            .await?;

        // user1 non dovrebbe più avere metadata
        assert!(repo.read(&(user1_id, chat1_id)).await?.is_none());
        assert!(repo.read(&(user1_id, chat2_id)).await?.is_none());

        // user2 dovrebbe ancora avere i suoi metadata
        assert!(repo.read(&(user2_id, chat1_id)).await?.is_some());
        assert!(repo.read(&(user2_id, chat2_id)).await?.is_some());

        // Elimina chat1
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", chat1_id)
            .execute(&pool)
            .await?;

        // Nessuno dovrebbe avere metadata per chat1
        assert!(repo.read(&(user2_id, chat1_id)).await?.is_none());

        // user2 in chat2 dovrebbe ancora esistere
        let remaining = repo.read(&(user2_id, chat2_id)).await?;
        assert!(remaining.is_some());
        assert_eq!(remaining.unwrap().user_role, Some(UserRole::Admin));

        Ok(())
    }

    /// Test NEGATIVO: update con combinazioni di ID validi e invalidi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_mixed_valid_invalid_ids(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        // User valido, chat invalida
        let result1 = repo.update(&(1, 999), &update_dto).await;
        assert!(result1.is_err());

        // User invalido, chat valida
        let result2 = repo.update(&(999, 1), &update_dto).await;
        assert!(result2.is_err());

        // User negativo, chat valida
        let result3 = repo.update(&(-5, 1), &update_dto).await;
        assert!(result3.is_err());

        // User valido, chat negativa
        let result4 = repo.update(&(1, -5), &update_dto).await;
        assert!(result4.is_err());

        Ok(())
    }

    /// Test NEGATIVO: update ripetuti dopo CASCADE DELETE falliscono
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_multiple_times_after_cascade(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Bob è nella General Chat
        assert!(repo.read(&(2, 1)).await?.is_some());

        // Elimina Bob
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;

        let update_dto = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        // Tenta update multiple volte - dovrebbero tutti fallire
        for _ in 0..3 {
            let result = repo.update(&(2, 1), &update_dto).await;
            assert!(
                result.is_err(),
                "L'update dovrebbe sempre fallire dopo CASCADE DELETE"
            );
        }

        Ok(())
    }

    /// Test NEGATIVO: update dopo operazione fallita non influenza metadata esistenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_update_isolation_after_failed_update(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Tenta update su metadata inesistente
        let invalid_update = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result = repo.update(&(999, 1), &invalid_update).await;
        assert!(result.is_err(), "L'update dovrebbe fallire");

        // Verifica che i metadata esistenti siano ancora accessibili e aggiornabili
        let valid_update = UpdateUserChatMetadataDTO {
            user_role: Some(UserRole::Admin),
            messages_visible_from: None,
            messages_received_until: None,
        };

        let result2 = repo.update(&(1, 1), &valid_update).await;
        assert!(result2.is_ok(), "L'update valido dovrebbe avere successo");

        let updated = result2.unwrap();
        assert_eq!(updated.user_role, Some(UserRole::Admin));

        Ok(())
    }

    /*------------------------------------*/
    /* Unit tests: delete (casi negativi) */
    /*------------------------------------*/

    /// Test NEGATIVO: delete di metadata inesistente (non genera errore ma non elimina nulla)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_not_exists(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Bob (user_id=2) non è nella Dev Team (chat_id=3)
        let result = repo.delete(&(2, 3)).await;

        // Delete non genera errore anche se il record non esiste
        assert!(
            result.is_ok(),
            "Delete dovrebbe avere successo anche se il record non esiste"
        );

        // Verifica che i metadata esistenti non siano stati toccati
        let existing = repo.read(&(2, 1)).await?;
        assert!(
            existing.is_some(),
            "I metadata esistenti non dovrebbero essere toccati"
        );

        Ok(())
    }

    /// Test NEGATIVO: delete con user_id inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_invalid_user_id(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        let result = repo.delete(&(999, 1)).await;

        assert!(
            result.is_ok(),
            "Delete non genera errore per user_id inesistente"
        );

        // Verifica che i metadata della chat siano intatti
        let members = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(
            members.len(),
            3,
            "I membri della General Chat dovrebbero essere intatti"
        );

        Ok(())
    }

    /// Test NEGATIVO: delete con chat_id inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_invalid_chat_id(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        let result = repo.delete(&(1, 999)).await;

        assert!(
            result.is_ok(),
            "Delete non genera errore per chat_id inesistente"
        );

        // Verifica che i metadata di Alice siano intatti
        let alice_chats = repo.find_many_by_user_id(&1).await?;
        assert_eq!(
            alice_chats.len(),
            3,
            "Le chat di Alice dovrebbero essere intatte"
        );

        Ok(())
    }

    /// Test NEGATIVO: delete con entrambi user_id e chat_id inesistenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_both_invalid(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let result = repo.delete(&(888, 999)).await;

        assert!(
            result.is_ok(),
            "Delete non genera errore anche per entrambi gli ID inesistenti"
        );

        Ok(())
    }

    /// Test NEGATIVO: delete con ID negativi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_negative_ids(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let result = repo.delete(&(-1, -1)).await;

        assert!(result.is_ok(), "Delete non genera errore per ID negativi");

        Ok(())
    }

    /// Test NEGATIVO: delete con ID zero
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_zero_ids(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        let result = repo.delete(&(0, 0)).await;

        assert!(result.is_ok(), "Delete non genera errore per ID zero");

        Ok(())
    }

    /// Test NEGATIVO: delete dopo che l'utente è già stato eliminato (CASCADE)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_after_user_cascade_deleted(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Bob è nella General Chat
        let before = repo.read(&(2, 1)).await?;
        assert!(before.is_some());

        // Elimina Bob - CASCADE elimina automaticamente il metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", 2)
            .execute(&pool)
            .await?;

        // Verifica che sia stato eliminato
        assert!(repo.read(&(2, 1)).await?.is_none());

        // Tentativo di delete non genera errore (record già inesistente)
        let result = repo.delete(&(2, 1)).await;
        assert!(
            result.is_ok(),
            "Delete dovrebbe avere successo anche se già eliminato da CASCADE"
        );

        Ok(())
    }

    /// Test NEGATIVO: delete dopo che la chat è già stata eliminata (CASCADE)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_after_chat_cascade_deleted(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Alice è nella General Chat
        let before = repo.read(&(1, 1)).await?;
        assert!(before.is_some());

        // Elimina la General Chat - CASCADE elimina tutti i metadata
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", 1)
            .execute(&pool)
            .await?;

        // Verifica che sia stato eliminato
        assert!(repo.read(&(1, 1)).await?.is_none());

        // Tentativo di delete non genera errore
        let result = repo.delete(&(1, 1)).await;
        assert!(
            result.is_ok(),
            "Delete dovrebbe avere successo anche se già eliminato da CASCADE"
        );

        Ok(())
    }

    /// Test NEGATIVO: delete doppio dello stesso metadata
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_twice_same_metadata(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un metadata
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&dto).await?;

        // Verifica che esista
        assert!(repo.read(&(1, new_chat_id)).await?.is_some());

        // Prima delete
        let result1 = repo.delete(&(1, new_chat_id)).await;
        assert!(result1.is_ok());

        // Verifica che sia stato eliminato
        assert!(repo.read(&(1, new_chat_id)).await?.is_none());

        // Seconda delete sullo stesso metadata (già inesistente)
        let result2 = repo.delete(&(1, new_chat_id)).await;
        assert!(
            result2.is_ok(),
            "Delete dovrebbe avere successo anche la seconda volta"
        );

        Ok(())
    }

    /// Test NEGATIVO CASCADE: verifica che dopo delete dell'utente, i metadata siano eliminati
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_verify_cascade_on_user(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un nuovo utente con metadata in più chat
        let new_user_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "testuser",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();

        // Aggiungi l'utente a 2 chat
        for &chat_id in &[1, 2] {
            let dto = CreateUserChatMetadataDTO {
                user_id: new_user_id,
                chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            };
            repo.create(&dto).await?;
        }

        // Verifica che l'utente sia in 2 chat
        let user_chats = repo.find_many_by_user_id(&new_user_id).await?;
        assert_eq!(user_chats.len(), 2);

        // Elimina l'utente - CASCADE dovrebbe eliminare automaticamente tutti i metadata
        sqlx::query!("DELETE FROM users WHERE user_id = ?", new_user_id)
            .execute(&pool)
            .await?;

        // Verifica che tutti i metadata siano stati eliminati da CASCADE
        let user_chats_after = repo.find_many_by_user_id(&new_user_id).await?;
        assert_eq!(
            user_chats_after.len(),
            0,
            "Tutti i metadata dovrebbero essere eliminati da CASCADE"
        );

        // Tentativo di delete manuale non genera errori
        let result1 = repo.delete(&(new_user_id, 1)).await;
        let result2 = repo.delete(&(new_user_id, 2)).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        Ok(())
    }

    /// Test NEGATIVO CASCADE: verifica che dopo delete della chat, tutti i membri siano eliminati
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_verify_cascade_on_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea una nuova chat con 3 membri
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Group",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();

        for &user_id in &[1, 2, 3] {
            let dto = CreateUserChatMetadataDTO {
                user_id,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            };
            repo.create(&dto).await?;
        }

        // Verifica che ci siano 3 membri
        let members = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(members.len(), 3);

        // Elimina la chat - CASCADE dovrebbe eliminare tutti i metadata
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", new_chat_id)
            .execute(&pool)
            .await?;

        // Verifica che tutti i metadata siano stati eliminati da CASCADE
        let members_after = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(
            members_after.len(),
            0,
            "Tutti i membri dovrebbero essere eliminati da CASCADE"
        );

        // Tentativo di delete manuale non genera errori
        for &user_id in &[1, 2, 3] {
            let result = repo.delete(&(user_id, new_chat_id)).await;
            assert!(result.is_ok());
        }

        Ok(())
    }

    /// Test NEGATIVO: delete in ordine diverso non causa problemi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_various_orders(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea 3 nuovi metadata
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();

        for &user_id in &[1, 2, 3] {
            let dto = CreateUserChatMetadataDTO {
                user_id,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            };
            repo.create(&dto).await?;
        }

        // Delete in ordine sparso
        repo.delete(&(2, new_chat_id)).await?;
        repo.delete(&(1, new_chat_id)).await?;
        repo.delete(&(3, new_chat_id)).await?;

        // Verifica che tutti siano stati eliminati
        let members = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(members.len(), 0);

        Ok(())
    }

    /// Test NEGATIVO: delete non influenza altre chat dello stesso utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_does_not_affect_other_chats(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Alice è in 3 chat
        let initial_count = repo.find_many_by_user_id(&1).await?.len();
        assert_eq!(initial_count, 3);

        // Delete Alice dalla General Chat
        repo.delete(&(1, 1)).await?;

        // Alice dovrebbe essere ancora in 2 chat
        let after_delete = repo.find_many_by_user_id(&1).await?;
        assert_eq!(after_delete.len(), 2);

        // Verifica che sia stata eliminata solo dalla General Chat
        assert!(repo.read(&(1, 1)).await?.is_none());
        assert!(repo.read(&(1, 2)).await?.is_some());
        assert!(repo.read(&(1, 3)).await?.is_some());

        Ok(())
    }

    /// Test NEGATIVO: delete non influenza altri utenti nella stessa chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_does_not_affect_other_users(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // General Chat ha 3 membri
        let initial_members = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(initial_members.len(), 3);

        // Delete Bob dalla General Chat
        repo.delete(&(2, 1)).await?;

        // Dovrebbero rimanere 2 membri
        let after_delete = repo.find_many_by_chat_id(&1).await?;
        assert_eq!(after_delete.len(), 2);

        // Verifica che solo Bob sia stato eliminato
        assert!(repo.read(&(1, 1)).await?.is_some());
        assert!(repo.read(&(2, 1)).await?.is_none());
        assert!(repo.read(&(3, 1)).await?.is_some());

        Ok(())
    }

    /// Test NEGATIVO CASCADE: scenario complesso con delete manuali e CASCADE
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_cascade_complex_scenario(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea 2 utenti e 2 chat con matrice completa
        let user1_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user1",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let user2_id = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            "user2",
            "password"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let chat1_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Chat 1",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let chat2_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Chat 2",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();

        // Crea 4 metadata
        for &user_id in &[user1_id, user2_id] {
            for &chat_id in &[chat1_id, chat2_id] {
                let dto = CreateUserChatMetadataDTO {
                    user_id,
                    chat_id,
                    user_role: Some(UserRole::Member),
                    member_since: now,
                    messages_visible_from: now,
                    messages_received_until: now,
                };
                repo.create(&dto).await?;
            }
        }

        // Verifica: 4 metadata esistono
        assert!(repo.read(&(user1_id, chat1_id)).await?.is_some());
        assert!(repo.read(&(user1_id, chat2_id)).await?.is_some());
        assert!(repo.read(&(user2_id, chat1_id)).await?.is_some());
        assert!(repo.read(&(user2_id, chat2_id)).await?.is_some());

        // Delete manuale di user1 da chat1
        repo.delete(&(user1_id, chat1_id)).await?;
        assert!(repo.read(&(user1_id, chat1_id)).await?.is_none());

        // Elimina user1 - CASCADE elimina user1-chat2
        sqlx::query!("DELETE FROM users WHERE user_id = ?", user1_id)
            .execute(&pool)
            .await?;

        assert!(repo.read(&(user1_id, chat2_id)).await?.is_none());

        // user2 dovrebbe avere ancora entrambi i metadata
        assert!(repo.read(&(user2_id, chat1_id)).await?.is_some());
        assert!(repo.read(&(user2_id, chat2_id)).await?.is_some());

        // Elimina chat1 - CASCADE elimina user2-chat1
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", chat1_id)
            .execute(&pool)
            .await?;

        assert!(repo.read(&(user2_id, chat1_id)).await?.is_none());

        // Solo user2-chat2 dovrebbe rimanere
        assert!(repo.read(&(user2_id, chat2_id)).await?.is_some());

        Ok(())
    }

    /// Test NEGATIVO: delete con combinazioni di ID validi e invalidi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_mixed_valid_invalid_ids(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);

        // Tutte queste delete non dovrebbero generare errori
        assert!(repo.delete(&(1, 999)).await.is_ok());
        assert!(repo.delete(&(999, 1)).await.is_ok());
        assert!(repo.delete(&(-5, 1)).await.is_ok());
        assert!(repo.delete(&(1, -5)).await.is_ok());
        assert!(repo.delete(&(0, 0)).await.is_ok());

        Ok(())
    }

    /// Test NEGATIVO: delete ripetute non causano errori
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_multiple_times_no_error(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Crea un metadata
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test Chat",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let now = chrono::Utc::now();
        let dto = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&dto).await?;

        // Delete multiple volte
        for _ in 0..5 {
            let result = repo.delete(&(1, new_chat_id)).await;
            assert!(result.is_ok(), "Delete dovrebbe sempre avere successo");
        }

        // Verifica che sia stato eliminato
        assert!(repo.read(&(1, new_chat_id)).await?.is_none());

        Ok(())
    }

    /// Test NEGATIVO: delete dopo operazioni fallite non causa problemi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_delete_after_failed_operations(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool.clone());

        // Tenta create con FK invalida (fallisce)
        let now = chrono::Utc::now();
        let invalid_dto = CreateUserChatMetadataDTO {
            user_id: 999,
            chat_id: 1,
            user_role: Some(UserRole::Member),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        let create_result = repo.create(&invalid_dto).await;
        assert!(create_result.is_err());

        // Delete non dovrebbe generare errori
        let delete_result = repo.delete(&(999, 1)).await;
        assert!(delete_result.is_ok());

        // Verifica che i metadata esistenti siano ancora validi
        let existing = repo.read(&(1, 1)).await?;
        assert!(existing.is_some());

        // Delete valido dovrebbe funzionare
        let new_chat_id = sqlx::query!(
            "INSERT INTO chats (title, description, chat_type) VALUES (?, ?, ?)",
            "Test",
            None::<String>,
            "GROUP"
        )
        .execute(&pool)
        .await?
        .last_insert_id() as i32;

        let valid_dto = CreateUserChatMetadataDTO {
            user_id: 1,
            chat_id: new_chat_id,
            user_role: Some(UserRole::Owner),
            member_since: now,
            messages_visible_from: now,
            messages_received_until: now,
        };

        repo.create(&valid_dto).await?;

        let delete_valid = repo.delete(&(1, new_chat_id)).await;
        assert!(delete_valid.is_ok());
        assert!(repo.read(&(1, new_chat_id)).await?.is_none());

        Ok(())
    }
}
