//! InvitationRepository - Repository per la gestione degli inviti

use super::{Create, Delete, Read, Update};
use crate::dtos::{CreateInvitationDTO, UpdateInvitationDTO};
use crate::entities::{Invitation, InvitationStatus};
use sqlx::{Error, MySqlPool};

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

    //MOD: controllo prima di inviare invito
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
    async fn create(&self, data: &CreateInvitationDTO) -> Result<Invitation, Error> {
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
    use sqlx::MySqlPool;

    /// Test generico - esempio di utilizzo di #[sqlx::test]
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users", "chats", "invitations")))]
    async fn test_example(_pool: MySqlPool) -> sqlx::Result<()> {
        // Il database è stato creato automaticamente con migrations applicate
        // I fixtures sono stati caricati in ordine: users, chats, invitations
        // Implementa qui i tuoi test per InvitationRepository
        Ok(())
    }
}
