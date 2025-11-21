//! InvitationRepository - Repository per la gestione degli inviti

use super::{Create, Delete, Read, Update};
use crate::dtos::{CreateInvitationDTO, UpdateInvitationDTO};
use crate::entities::{Invitation, InvitationStatus};
use sqlx::{Error, MySqlPool};
use tracing::{debug, info, instrument};

//INVITATION REPOSITORY
pub struct InvitationRepository {
    connection_pool: MySqlPool,
}

impl InvitationRepository {
    pub fn new(connection_pool: MySqlPool) -> Self {
        Self { connection_pool }
    }

    /// Get all pending invitations for a specific user
    pub async fn find_many_by_user_id(&self, user_id: &i32) -> Result<Vec<Invitation>, Error> {
        let invitations = sqlx::query_as!(
            Invitation,
            r#"
            SELECT 
                invite_id,
                target_chat_id,
                invited_id,
                invitee_id,
                state as "state: InvitationStatus",
                created_at
            FROM invitations 
            WHERE invited_id = ? AND state = 'PENDING'
            "#,
            user_id
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(invitations)
    }

    /// Check if there's already a pending invitation for user to chat
    pub async fn has_pending_invitation(
        &self,
        user_id: &i32,
        chat_id: &i32,
    ) -> Result<bool, Error> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM invitations WHERE invited_id = ? AND target_chat_id = ? AND state = 'PENDING'",
            user_id,
            chat_id
        )
        .fetch_one(&self.connection_pool)
        .await?;

        Ok(count.count > 0)
    }
}

impl Create<Invitation, CreateInvitationDTO> for InvitationRepository {
    #[instrument(skip(self, data), fields(chat_id = %data.target_chat_id, inviter = %data.invitee_id, invited = %data.invited_id))]
    async fn create(&self, data: &CreateInvitationDTO) -> Result<Invitation, Error> {
        debug!("Creating new invitation");
        // Insert invitation using MySQL syntax
        // state e created_at vengono gestiti dal database (default: Pending e NOW())
        let now = chrono::Utc::now();
        let state = InvitationStatus::Pending; // default state

        let result = sqlx::query!(
            r#"
            INSERT INTO invitations (target_chat_id, invited_id, invitee_id, state, created_at) 
            VALUES (?, ?, ?, ?, ?)
            "#,
            data.target_chat_id,
            data.invited_id,
            data.invitee_id,
            state,
            now
        )
        .execute(&self.connection_pool)
        .await?;

        // Get the last inserted ID
        let new_id = result.last_insert_id() as i32;

        info!("Invitation created with id {}", new_id);

        // Return the created invitation with the new ID
        Ok(Invitation {
            invite_id: new_id,
            target_chat_id: data.target_chat_id,
            invited_id: data.invited_id,
            invitee_id: data.invitee_id,
            state,
            created_at: now,
        })
    }
}

impl Read<Invitation, i32> for InvitationRepository {
    async fn read(&self, id: &i32) -> Result<Option<Invitation>, Error> {
        let invitation = sqlx::query_as!(
            Invitation,
            r#"
            SELECT 
                invite_id,
                target_chat_id,
                invited_id,
                invitee_id,
                state as "state: InvitationStatus",
                created_at
            FROM invitations 
            WHERE invite_id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(invitation)
    }
}

impl Update<Invitation, UpdateInvitationDTO, i32> for InvitationRepository {
    async fn update(&self, id: &i32, data: &UpdateInvitationDTO) -> Result<Invitation, Error> {
        // First, get the current invitation to ensure it exists
        let current_invitation = self
            .read(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // If no state to update, return current invitation
        if data.state.is_none() {
            return Ok(current_invitation);
        }

        // Update invitation state
        sqlx::query!(
            "UPDATE invitations SET state = ? WHERE invite_id = ?",
            data.state,
            id
        )
        .execute(&self.connection_pool)
        .await?;

        // Fetch and return the updated invitation
        self.read(id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
    }
}

impl Delete<i32> for InvitationRepository {
    async fn delete(&self, id: &i32) -> Result<(), Error> {
        sqlx::query!("DELETE FROM invitations WHERE invite_id = ?", id)
            .execute(&self.connection_pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::MySqlPool;

    // ============================================================================
    // Tests for find_many_by_user_id method
    // ============================================================================

    /// Test: verifica che find_many_by_user_id restituisca solo inviti PENDING
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_find_many_by_user_id_returns_only_pending(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Assumendo che il fixture "invitations" contenga inviti con invited_id = 1
        let user_id = 1;
        let invitations = repo.find_many_by_user_id(&user_id).await?;

        // Verifica che tutti gli inviti restituiti siano PENDING
        for inv in &invitations {
            assert_eq!(inv.state, InvitationStatus::Pending);
            assert_eq!(inv.invited_id, user_id);
        }

        Ok(())
    }

    /// Test: verifica che restituisca un array vuoto per utenti senza inviti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_find_many_by_user_id_returns_empty_when_no_invitations(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let user_id = 999; // utente senza inviti
        let invitations = repo.find_many_by_user_id(&user_id).await?;

        assert!(invitations.is_empty());
        Ok(())
    }

    /// Test: verifica che gli inviti ACCEPTED/REJECTED siano esclusi
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_find_many_by_user_id_excludes_non_pending_invitations(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Crea un invito PENDING
        let user_id = 2;
        let invite = CreateInvitationDTO {
            target_chat_id: 1,
            invited_id: user_id,
            invitee_id: 3,
        };

        let created = repo.create(&invite).await?;

        // Verifica che l'invito PENDING venga restituito
        let invitations_pending = repo.find_many_by_user_id(&user_id).await?;
        assert!(
            invitations_pending
                .iter()
                .any(|inv| inv.invite_id == created.invite_id)
        );

        // Aggiorna lo stato ad ACCEPTED
        sqlx::query!(
            "UPDATE invitations SET state = 'ACCEPTED' WHERE invite_id = ?",
            created.invite_id
        )
        .execute(&pool)
        .await?;

        // Verifica che l'invito ACCEPTED non venga restituito
        let invitations_after = repo.find_many_by_user_id(&user_id).await?;
        assert!(
            !invitations_after
                .iter()
                .any(|inv| inv.invite_id == created.invite_id)
        );

        Ok(())
    }

    /// Test: verifica il comportamento CASCADE quando viene eliminata una chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_find_many_by_user_id_cascade_on_chat_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let user_id = 1;
        let chat_id = 1;

        // Crea un invito per la chat
        let invite = CreateInvitationDTO {
            target_chat_id: chat_id,
            invited_id: user_id,
            invitee_id: 2,
        };

        let created = repo.create(&invite).await?;

        // Verifica che l'invito esista
        let invitations_before = repo.find_many_by_user_id(&user_id).await?;
        assert!(
            invitations_before
                .iter()
                .any(|inv| inv.invite_id == created.invite_id)
        );

        // Elimina la chat (se configurato CASCADE DELETE nella FK)
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", chat_id)
            .execute(&pool)
            .await?;

        // Verifica che gli inviti per quella chat siano stati eliminati in cascata
        let invitations_after = repo.find_many_by_user_id(&user_id).await?;
        assert!(
            !invitations_after
                .iter()
                .any(|inv| inv.target_chat_id == chat_id)
        );

        Ok(())
    }

    /// Test: verifica il comportamento CASCADE quando viene eliminato l'utente invitante (invitee)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_find_many_by_user_id_cascade_on_inviter_delete(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let inviter_id = 1;
        let invited_id = 2;

        // Crea un invito dove user 1 invita user 2
        let invite = CreateInvitationDTO {
            target_chat_id: 1,
            invited_id,
            invitee_id: inviter_id,
        };

        let created = repo.create(&invite).await?;

        // Verifica che l'invito esista
        let invitations_before = repo.find_many_by_user_id(&invited_id).await?;
        assert!(
            invitations_before
                .iter()
                .any(|inv| inv.invite_id == created.invite_id)
        );

        // Elimina l'utente invitante (se configurato CASCADE DELETE nella FK)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", inviter_id)
            .execute(&pool)
            .await?;

        // Verifica che gli inviti da quell'utente siano stati eliminati in cascata
        let invitations_after = repo.find_many_by_user_id(&invited_id).await?;
        assert!(
            !invitations_after
                .iter()
                .any(|inv| inv.invitee_id == inviter_id)
        );

        Ok(())
    }

    /// Test: verifica il comportamento CASCADE quando viene eliminato l'utente invitato (invited)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_find_many_by_user_id_cascade_on_invited_delete(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let inviter_id = 1;
        let invited_id = 2;

        // Crea un invito dove user 1 invita user 2
        let invite = CreateInvitationDTO {
            target_chat_id: 1,
            invited_id,
            invitee_id: inviter_id,
        };

        repo.create(&invite).await?;

        // Elimina l'utente invitato (se configurato CASCADE DELETE nella FK)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", invited_id)
            .execute(&pool)
            .await?;

        // Verifica che gli inviti per quell'utente siano stati eliminati in cascata
        // (non dovrebbe esserci nessun invito restituito)
        let invitations_after = repo.find_many_by_user_id(&invited_id).await?;
        assert!(invitations_after.is_empty());

        Ok(())
    }

    /// Test: verifica che restituisca correttamente più inviti PENDING per lo stesso utente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_find_many_by_user_id_multiple_pending_invitations(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let user_id = 3;
        let mut created_ids = Vec::new();

        // Crea più inviti PENDING per lo stesso utente
        for chat_id in 2..=3 {
            let invite = CreateInvitationDTO {
                target_chat_id: chat_id,
                invited_id: user_id,
                invitee_id: 1,
            };
            let created = repo.create(&invite).await?;
            created_ids.push(created.invite_id);
        }

        let invitations = repo.find_many_by_user_id(&user_id).await?;

        // Verifica che tutti gli inviti creati siano restituiti
        for created_id in &created_ids {
            assert!(invitations.iter().any(|inv| inv.invite_id == *created_id));
        }

        // Verifica che tutti siano PENDING e per l'utente corretto
        for inv in invitations
            .iter()
            .filter(|inv| created_ids.contains(&inv.invite_id))
        {
            assert_eq!(inv.invited_id, user_id);
            assert_eq!(inv.state, InvitationStatus::Pending);
        }

        Ok(())
    }

    // ============================================================================
    // Tests for has_pending_invitation method
    // ============================================================================

    /// Test: verifica che has_pending_invitation ritorni true per inviti PENDING esistenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_has_pending_invitation_returns_true_when_pending_exists(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: Bob invita Charlie (user_id=3) al General Chat (chat_id=1) con stato PENDING
        let user_id = 3;
        let chat_id = 1;

        let has_pending = repo.has_pending_invitation(&user_id, &chat_id).await?;

        assert!(has_pending, "Expected pending invitation to exist");

        Ok(())
    }

    /// Test: verifica che has_pending_invitation ritorni false quando non ci sono inviti PENDING
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_has_pending_invitation_returns_false_when_no_pending(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // User e chat che non hanno inviti PENDING
        let user_id = 1;
        let chat_id = 2;

        let has_pending = repo.has_pending_invitation(&user_id, &chat_id).await?;

        assert!(!has_pending, "Expected no pending invitation");

        Ok(())
    }

    /// Test: verifica che has_pending_invitation escluda inviti ACCEPTED
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_has_pending_invitation_excludes_accepted_invitations(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: Alice invita Bob (user_id=2) al Dev Team (chat_id=3) con stato ACCEPTED
        let user_id = 2;
        let chat_id = 3;

        let has_pending = repo.has_pending_invitation(&user_id, &chat_id).await?;

        assert!(
            !has_pending,
            "Expected no pending invitation (only ACCEPTED exists)"
        );

        Ok(())
    }

    /// Test: verifica che has_pending_invitation escluda inviti REJECTED
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_has_pending_invitation_excludes_rejected_invitations(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: Charlie invita Alice (user_id=1) al General Chat (chat_id=1) con stato REJECTED
        let user_id = 1;
        let chat_id = 1;

        let has_pending = repo.has_pending_invitation(&user_id, &chat_id).await?;

        assert!(
            !has_pending,
            "Expected no pending invitation (only REJECTED exists)"
        );

        Ok(())
    }

    /// Test: verifica che has_pending_invitation passi da true a false quando l'invito viene accettato
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_has_pending_invitation_changes_after_state_update(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let user_id = 2;
        let chat_id = 1;

        // Crea un invito PENDING
        let invite = CreateInvitationDTO {
            target_chat_id: chat_id,
            invited_id: user_id,
            invitee_id: 1,
        };

        let created = repo.create(&invite).await?;

        // Verifica che sia PENDING
        let has_pending_before = repo.has_pending_invitation(&user_id, &chat_id).await?;
        assert!(
            has_pending_before,
            "Expected pending invitation after creation"
        );

        // Aggiorna lo stato ad ACCEPTED
        sqlx::query!(
            "UPDATE invitations SET state = 'ACCEPTED' WHERE invite_id = ?",
            created.invite_id
        )
        .execute(&pool)
        .await?;

        // Verifica che non ci sia più un invito PENDING
        let has_pending_after = repo.has_pending_invitation(&user_id, &chat_id).await?;
        assert!(
            !has_pending_after,
            "Expected no pending invitation after acceptance"
        );

        Ok(())
    }

    /// Test: verifica il comportamento CASCADE quando viene eliminata la chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_has_pending_invitation_cascade_on_chat_delete(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let user_id = 3;
        let chat_id = 1;

        // Verifica che esista un invito PENDING dal fixture
        let has_pending_before = repo.has_pending_invitation(&user_id, &chat_id).await?;
        assert!(
            has_pending_before,
            "Expected pending invitation before chat deletion"
        );

        // Elimina la chat (CASCADE DELETE dovrebbe eliminare gli inviti)
        sqlx::query!("DELETE FROM chats WHERE chat_id = ?", chat_id)
            .execute(&pool)
            .await?;

        // Verifica che non ci sia più l'invito PENDING
        let has_pending_after = repo.has_pending_invitation(&user_id, &chat_id).await?;
        assert!(
            !has_pending_after,
            "Expected no pending invitation after chat deletion"
        );

        Ok(())
    }

    /// Test: verifica il comportamento CASCADE quando viene eliminato l'utente invitato
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_has_pending_invitation_cascade_on_invited_user_delete(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let user_id = 3;
        let chat_id = 1;

        // Verifica che esista un invito PENDING dal fixture
        let has_pending_before = repo.has_pending_invitation(&user_id, &chat_id).await?;
        assert!(
            has_pending_before,
            "Expected pending invitation before user deletion"
        );

        // Elimina l'utente invitato (CASCADE DELETE dovrebbe eliminare gli inviti)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", user_id)
            .execute(&pool)
            .await?;

        // Verifica che non ci sia più l'invito PENDING
        let has_pending_after = repo.has_pending_invitation(&user_id, &chat_id).await?;
        assert!(
            !has_pending_after,
            "Expected no pending invitation after user deletion"
        );

        Ok(())
    }

    /// Test: verifica il comportamento CASCADE quando viene eliminato l'utente invitante
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_has_pending_invitation_cascade_on_inviter_delete(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let invited_user_id = 3;
        let inviter_user_id = 2; // Bob è l'invitante
        let chat_id = 1;

        // Verifica che esista un invito PENDING dal fixture (Bob invita Charlie)
        let has_pending_before = repo
            .has_pending_invitation(&invited_user_id, &chat_id)
            .await?;
        assert!(
            has_pending_before,
            "Expected pending invitation before inviter deletion"
        );

        // Elimina l'utente invitante (CASCADE DELETE dovrebbe eliminare gli inviti creati da lui)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", inviter_user_id)
            .execute(&pool)
            .await?;

        // Verifica che non ci sia più l'invito PENDING
        let has_pending_after = repo
            .has_pending_invitation(&invited_user_id, &chat_id)
            .await?;
        assert!(
            !has_pending_after,
            "Expected no pending invitation after inviter deletion"
        );

        Ok(())
    }

    /// Test: verifica che has_pending_invitation gestisca correttamente utenti inesistenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_has_pending_invitation_with_nonexistent_user(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let nonexistent_user_id = 9999;
        let chat_id = 1;

        let has_pending = repo
            .has_pending_invitation(&nonexistent_user_id, &chat_id)
            .await?;

        assert!(
            !has_pending,
            "Expected no pending invitation for nonexistent user"
        );

        Ok(())
    }

    /// Test: verifica che has_pending_invitation gestisca correttamente chat inesistenti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_has_pending_invitation_with_nonexistent_chat(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let user_id = 1;
        let nonexistent_chat_id = 9999;

        let has_pending = repo
            .has_pending_invitation(&user_id, &nonexistent_chat_id)
            .await?;

        assert!(
            !has_pending,
            "Expected no pending invitation for nonexistent chat"
        );

        Ok(())
    }

    /// Test: verifica che has_pending_invitation rispetti la UNIQUE constraint (non duplicati PENDING)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_has_pending_invitation_unique_constraint(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let user_id = 3;
        let chat_id = 1;

        // Verifica che esista un invito PENDING
        let has_pending = repo.has_pending_invitation(&user_id, &chat_id).await?;
        assert!(has_pending);

        // Prova a creare un duplicato (dovrebbe fallire per la UNIQUE constraint)
        let duplicate_invite = CreateInvitationDTO {
            target_chat_id: chat_id,
            invited_id: user_id,
            invitee_id: 1,
        };

        let result = repo.create(&duplicate_invite).await;

        // Verifica che l'inserimento fallisca
        assert!(result.is_err(), "Expected duplicate invitation to fail");

        // Verifica che ci sia ancora solo un invito PENDING
        let still_has_pending = repo.has_pending_invitation(&user_id, &chat_id).await?;
        assert!(still_has_pending);

        Ok(())
    }

    // ============================================================================
    // Tests for CREATE method
    // ============================================================================

    /// Test: verifica che create crei correttamente un nuovo invito PENDING
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_invitation_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let invite_dto = CreateInvitationDTO {
            target_chat_id: 1,
            invited_id: 2,
            invitee_id: 1,
        };

        let created = repo.create(&invite_dto).await?;

        // Verifica che l'invito sia stato creato con i dati corretti
        assert!(created.invite_id > 0);
        assert_eq!(created.target_chat_id, invite_dto.target_chat_id);
        assert_eq!(created.invited_id, invite_dto.invited_id);
        assert_eq!(created.invitee_id, invite_dto.invitee_id);
        assert_eq!(created.state, InvitationStatus::Pending);

        Ok(())
    }

    /// Test: verifica che create fallisca con FK violation per chat inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_invitation_fails_with_invalid_chat(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let invite_dto = CreateInvitationDTO {
            target_chat_id: 9999, // chat inesistente
            invited_id: 2,
            invitee_id: 1,
        };

        let result = repo.create(&invite_dto).await;

        assert!(
            result.is_err(),
            "Expected FK constraint violation for invalid chat_id"
        );

        Ok(())
    }

    /// Test: verifica che create fallisca con FK violation per invited_id inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_invitation_fails_with_invalid_invited_user(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let invite_dto = CreateInvitationDTO {
            target_chat_id: 1,
            invited_id: 9999, // utente inesistente
            invitee_id: 1,
        };

        let result = repo.create(&invite_dto).await;

        assert!(
            result.is_err(),
            "Expected FK constraint violation for invalid invited_id"
        );

        Ok(())
    }

    /// Test: verifica che create fallisca con FK violation per invitee_id inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_invitation_fails_with_invalid_inviter(
        pool: MySqlPool,
    ) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let invite_dto = CreateInvitationDTO {
            target_chat_id: 1,
            invited_id: 2,
            invitee_id: 9999, // utente inesistente
        };

        let result = repo.create(&invite_dto).await;

        assert!(
            result.is_err(),
            "Expected FK constraint violation for invalid invitee_id"
        );

        Ok(())
    }

    /// Test: verifica che create rispetti la UNIQUE constraint (target_chat_id, invited_id, state)
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_create_invitation_fails_with_duplicate(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture esiste già: (target_chat_id=1, invited_id=3, state=PENDING)
        let duplicate_dto = CreateInvitationDTO {
            target_chat_id: 1,
            invited_id: 3,
            invitee_id: 1,
        };

        let result = repo.create(&duplicate_dto).await;

        assert!(
            result.is_err(),
            "Expected unique constraint violation for duplicate invitation"
        );

        Ok(())
    }

    /// Test: verifica che si possano creare più inviti PENDING per lo stesso utente in chat diverse
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_create_multiple_invitations_different_chats(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let user_id = 2;

        // Crea inviti per chat diverse
        for chat_id in 1..=3 {
            let invite_dto = CreateInvitationDTO {
                target_chat_id: chat_id,
                invited_id: user_id,
                invitee_id: 1,
            };

            let created = repo.create(&invite_dto).await?;
            assert_eq!(created.target_chat_id, chat_id);
        }

        Ok(())
    }

    /// Test: verifica che si possa creare un invito PENDING se già esiste un invito con stato diverso
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_create_invitation_different_state_allowed(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: esiste (1, 3, PENDING)
        // Prima aggiorna lo stato esistente a REJECTED
        sqlx::query!(
            "UPDATE invitations SET state = 'REJECTED' WHERE target_chat_id = 1 AND invited_id = 3"
        )
        .execute(&pool)
        .await?;

        // Ora possiamo creare un nuovo PENDING (perché la UNIQUE è su chat+user+state)
        let new_invite = CreateInvitationDTO {
            target_chat_id: 1,
            invited_id: 3,
            invitee_id: 2,
        };

        let created = repo.create(&new_invite).await?;
        assert_eq!(created.state, InvitationStatus::Pending);

        Ok(())
    }

    // ============================================================================
    // Tests for READ method
    // ============================================================================

    /// Test: verifica che read restituisca un invito esistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_read_invitation_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id = 1
        let invite_id = 1;

        let invitation = repo.read(&invite_id).await?;

        assert!(invitation.is_some());
        let inv = invitation.unwrap();
        assert_eq!(inv.invite_id, invite_id);
        assert_eq!(inv.target_chat_id, 1);
        assert_eq!(inv.invited_id, 3);
        assert_eq!(inv.invitee_id, 2);
        assert_eq!(inv.state, InvitationStatus::Pending);

        Ok(())
    }

    /// Test: verifica che read restituisca None per invito inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_read_invitation_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let nonexistent_id = 9999;

        let invitation = repo.read(&nonexistent_id).await?;

        assert!(
            invitation.is_none(),
            "Expected None for nonexistent invitation"
        );

        Ok(())
    }

    /// Test: verifica che read restituisca l'invito dopo create
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats")))]
    async fn test_read_after_create(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let invite_dto = CreateInvitationDTO {
            target_chat_id: 1,
            invited_id: 2,
            invitee_id: 1,
        };

        let created = repo.create(&invite_dto).await?;

        let read_invitation = repo.read(&created.invite_id).await?;

        assert!(read_invitation.is_some());
        let inv = read_invitation.unwrap();
        assert_eq!(inv.invite_id, created.invite_id);
        assert_eq!(inv.target_chat_id, created.target_chat_id);
        assert_eq!(inv.invited_id, created.invited_id);

        Ok(())
    }

    /// Test: verifica CASCADE DELETE quando viene eliminata la chat
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_read_after_chat_cascade_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id=1 è associato a chat_id=1
        let invite_id = 1;

        // Verifica che l'invito esista
        let before = repo.read(&invite_id).await?;
        assert!(before.is_some());

        // Elimina la chat (CASCADE DELETE)
        sqlx::query!("DELETE FROM chats WHERE chat_id = 1")
            .execute(&pool)
            .await?;

        // Verifica che l'invito sia stato eliminato in cascata
        let after = repo.read(&invite_id).await?;
        assert!(
            after.is_none(),
            "Expected invitation to be deleted via CASCADE"
        );

        Ok(())
    }

    /// Test: verifica CASCADE DELETE quando viene eliminato l'utente invitato
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_read_after_invited_user_cascade_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id=1 ha invited_id=3
        let invite_id = 1;
        let invited_user_id = 3;

        // Verifica che l'invito esista
        let before = repo.read(&invite_id).await?;
        assert!(before.is_some());

        // Elimina l'utente invitato (CASCADE DELETE)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", invited_user_id)
            .execute(&pool)
            .await?;

        // Verifica che l'invito sia stato eliminato in cascata
        let after = repo.read(&invite_id).await?;
        assert!(
            after.is_none(),
            "Expected invitation to be deleted via CASCADE"
        );

        Ok(())
    }

    /// Test: verifica CASCADE DELETE quando viene eliminato l'utente invitante
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_read_after_inviter_cascade_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id=1 ha invitee_id=2 (Bob)
        let invite_id = 1;
        let inviter_user_id = 2;

        // Verifica che l'invito esista
        let before = repo.read(&invite_id).await?;
        assert!(before.is_some());

        // Elimina l'utente invitante (CASCADE DELETE)
        sqlx::query!("DELETE FROM users WHERE user_id = ?", inviter_user_id)
            .execute(&pool)
            .await?;

        // Verifica che l'invito sia stato eliminato in cascata
        let after = repo.read(&invite_id).await?;
        assert!(
            after.is_none(),
            "Expected invitation to be deleted via CASCADE"
        );

        Ok(())
    }

    // ============================================================================
    // Tests for UPDATE method
    // ============================================================================

    /// Test: verifica che update aggiorni correttamente lo stato da PENDING ad ACCEPTED
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_update_invitation_to_accepted(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id=1 è PENDING
        let invite_id = 1;

        let update_dto = UpdateInvitationDTO {
            state: Some(InvitationStatus::Accepted),
        };

        let updated = repo.update(&invite_id, &update_dto).await?;

        assert_eq!(updated.invite_id, invite_id);
        assert_eq!(updated.state, InvitationStatus::Accepted);

        Ok(())
    }

    /// Test: verifica che update aggiorni correttamente lo stato da PENDING a REJECTED
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_update_invitation_to_rejected(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id=1 è PENDING
        let invite_id = 1;

        let update_dto = UpdateInvitationDTO {
            state: Some(InvitationStatus::Rejected),
        };

        let updated = repo.update(&invite_id, &update_dto).await?;

        assert_eq!(updated.invite_id, invite_id);
        assert_eq!(updated.state, InvitationStatus::Rejected);

        Ok(())
    }

    /// Test: verifica che update con state=None non modifichi l'invito
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_update_invitation_with_no_state_change(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id=1 è PENDING
        let invite_id = 1;

        let before = repo.read(&invite_id).await?.unwrap();

        let update_dto = UpdateInvitationDTO { state: None };

        let updated = repo.update(&invite_id, &update_dto).await?;

        // Lo stato dovrebbe rimanere invariato
        assert_eq!(updated.state, before.state);

        Ok(())
    }

    /// Test: verifica che update fallisca per invito inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_update_invitation_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let nonexistent_id = 9999;

        let update_dto = UpdateInvitationDTO {
            state: Some(InvitationStatus::Accepted),
        };

        let result = repo.update(&nonexistent_id, &update_dto).await;

        assert!(result.is_err(), "Expected error for nonexistent invitation");

        Ok(())
    }

    /// Test: verifica che update possa cambiare stato più volte
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_update_invitation_multiple_times(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id=1 è PENDING
        let invite_id = 1;

        // Prima update: PENDING -> ACCEPTED
        let update1 = UpdateInvitationDTO {
            state: Some(InvitationStatus::Accepted),
        };
        let result1 = repo.update(&invite_id, &update1).await?;
        assert_eq!(result1.state, InvitationStatus::Accepted);

        // Seconda update: ACCEPTED -> REJECTED
        let update2 = UpdateInvitationDTO {
            state: Some(InvitationStatus::Rejected),
        };
        let result2 = repo.update(&invite_id, &update2).await?;
        assert_eq!(result2.state, InvitationStatus::Rejected);

        // Terza update: REJECTED -> PENDING
        let update3 = UpdateInvitationDTO {
            state: Some(InvitationStatus::Pending),
        };
        let result3 = repo.update(&invite_id, &update3).await?;
        assert_eq!(result3.state, InvitationStatus::Pending);

        Ok(())
    }

    /// Test: verifica che update preservi gli altri campi dell'invito
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_update_preserves_other_fields(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id=1
        let invite_id = 1;

        let before = repo.read(&invite_id).await?.unwrap();

        let update_dto = UpdateInvitationDTO {
            state: Some(InvitationStatus::Accepted),
        };

        let updated = repo.update(&invite_id, &update_dto).await?;

        // Verifica che solo lo stato sia cambiato
        assert_eq!(updated.invite_id, before.invite_id);
        assert_eq!(updated.target_chat_id, before.target_chat_id);
        assert_eq!(updated.invited_id, before.invited_id);
        assert_eq!(updated.invitee_id, before.invitee_id);
        assert_eq!(updated.created_at, before.created_at);
        assert_ne!(updated.state, before.state); // solo lo stato cambia

        Ok(())
    }

    // ============================================================================
    // Tests for DELETE method
    // ============================================================================

    /// Test: verifica che delete elimini correttamente un invito
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_delete_invitation_success(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: invite_id=1 esiste
        let invite_id = 1;

        // Verifica che esista
        let before = repo.read(&invite_id).await?;
        assert!(before.is_some());

        // Elimina
        repo.delete(&invite_id).await?;

        // Verifica che non esista più
        let after = repo.read(&invite_id).await?;
        assert!(after.is_none(), "Expected invitation to be deleted");

        Ok(())
    }

    /// Test: verifica che delete non fallisca per invito inesistente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_delete_invitation_not_found(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        let nonexistent_id = 9999;

        // DELETE su id inesistente non dovrebbe dare errore (affected rows = 0 è ok)
        let result = repo.delete(&nonexistent_id).await;

        assert!(
            result.is_ok(),
            "Expected delete to succeed even for nonexistent id"
        );

        Ok(())
    }

    /// Test: verifica che delete rimuova l'invito dal database
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_delete_removes_from_database(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Crea un nuovo invito
        let invite_dto = CreateInvitationDTO {
            target_chat_id: 2,
            invited_id: 2,
            invitee_id: 1,
        };

        let created = repo.create(&invite_dto).await?;
        let invite_id = created.invite_id;

        // Verifica che esista
        assert!(repo.read(&invite_id).await?.is_some());

        // Elimina
        repo.delete(&invite_id).await?;

        // Verifica che non sia più recuperabile
        assert!(repo.read(&invite_id).await?.is_none());

        // Verifica che non compaia in has_pending_invitation
        let has_pending = repo
            .has_pending_invitation(&invite_dto.invited_id, &invite_dto.target_chat_id)
            .await?;
        assert!(!has_pending);

        Ok(())
    }

    /// Test: verifica che delete multipli funzionino correttamente
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_delete_multiple_invitations(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: esistono inviti 1, 2, 3
        let invite_ids = vec![1, 2, 3];

        // Elimina tutti
        for id in &invite_ids {
            repo.delete(id).await?;
        }

        // Verifica che nessuno esista più
        for id in &invite_ids {
            let result = repo.read(id).await?;
            assert!(result.is_none(), "Expected invitation {} to be deleted", id);
        }

        Ok(())
    }

    /// Test: verifica che delete non influenzi altri inviti
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_delete_does_not_affect_other_invitations(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: esistono inviti 1, 2, 3
        let delete_id = 1;
        let keep_ids = vec![2, 3];

        // Elimina solo l'invito 1
        repo.delete(&delete_id).await?;

        // Verifica che 1 sia eliminato
        assert!(repo.read(&delete_id).await?.is_none());

        // Verifica che 2 e 3 esistano ancora
        for id in keep_ids {
            let result = repo.read(&id).await?;
            assert!(
                result.is_some(),
                "Expected invitation {} to still exist",
                id
            );
        }

        Ok(())
    }

    /// Test: verifica che si possa ricreare un invito dopo delete
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_recreate_after_delete(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = InvitationRepository::new(pool.clone());

        // Dal fixture: esiste (1, 3, PENDING)
        let original = repo.read(&1).await?.unwrap();

        // Elimina
        repo.delete(&1).await?;

        // Ricrea con gli stessi dati
        let recreate_dto = CreateInvitationDTO {
            target_chat_id: original.target_chat_id,
            invited_id: original.invited_id,
            invitee_id: original.invitee_id,
        };

        let recreated = repo.create(&recreate_dto).await?;

        // Verifica che sia stato ricreato (avrà un ID diverso)
        assert_ne!(recreated.invite_id, original.invite_id);
        assert_eq!(recreated.target_chat_id, original.target_chat_id);
        assert_eq!(recreated.invited_id, original.invited_id);
        assert_eq!(recreated.invitee_id, original.invitee_id);
        assert_eq!(recreated.state, InvitationStatus::Pending);

        Ok(())
    }
}
