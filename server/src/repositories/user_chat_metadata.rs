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
        assert_eq!(count.count, 0, "Tutti i metadata dell'utente dovrebbero essere eliminati (CASCADE)");
        
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
    async fn test_find_many_by_user_id_cascade_delete_multiple_chats(pool: MySqlPool) -> sqlx::Result<()> {
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
    async fn test_find_many_by_user_id_cascade_delete_user_with_multiple_chats(pool: MySqlPool) -> sqlx::Result<()> {
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
            1, 1
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
    async fn test_find_many_by_user_id_after_ownership_transfer(pool: MySqlPool) -> sqlx::Result<()> {
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
            2, 1
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
    async fn test_find_many_by_user_id_cascade_delete_all_user_chats(pool: MySqlPool) -> sqlx::Result<()> {
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
        assert!(user_exists.is_some(), "L'utente Bob dovrebbe esistere ancora");
        
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
    async fn test_find_many_by_user_id_cascade_mixed_operations(pool: MySqlPool) -> sqlx::Result<()> {
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
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: 1,
                chat_id: new_chat_id,
                user_role: Some(UserRole::Owner),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];
        
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
        assert_eq!(chat_members.len(), 0, "Nessun metadata dovrebbe essere creato (transazione rollback)");
        
        Ok(())
    }

    /// Test: atomicità - chat inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_many_atomicity_chat_not_exists(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserChatMetadataRepository::new(pool);
        
        let now = chrono::Utc::now();
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: 1,
                chat_id: 999, // Chat inesistente
                user_role: Some(UserRole::Owner),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];
        
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
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: 1,
                chat_id: 1, // Alice è già nella General Chat
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];
        
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
        let metadata_list = vec![
            CreateUserChatMetadataDTO {
                user_id: new_user_id,
                chat_id: 1,
                user_role: Some(UserRole::Member),
                member_since: now,
                messages_visible_from: now,
                messages_received_until: now,
            },
        ];
        
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
        assert!(metadata_after.is_none(), "Il metadata dovrebbe essere eliminato (CASCADE)");
        
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
        assert_eq!(members_after.len(), 0, "Tutti i metadata dovrebbero essere eliminati (CASCADE)");
        
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
        sqlx::query!("DELETE FROM users WHERE user_id IN (?, ?)", user4_id, user5_id)
            .execute(&pool)
            .await?;
        
        // Verifica che i metadata siano stati eliminati
        let members_after = repo.find_many_by_chat_id(&new_chat_id).await?;
        assert_eq!(members_after.len(), 0, "Tutti i metadata dovrebbero essere eliminati (CASCADE)");
        
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
