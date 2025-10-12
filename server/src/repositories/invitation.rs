//! InvitationRepository - Repository per la gestione degli inviti

use crate::entities::{Invitation, InvitationStatus};
use super::Crud;
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
    pub async fn get_pending_invitations_for_user(
        &self,
        user_id: &i32,
    ) -> Result<Vec<Invitation>, Error> {
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

    /// Update invitation status (accept/reject)
    pub async fn update_invitation_status(
        &self,
        invitation_id: &i32,
        new_status: &InvitationStatus,
    ) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE invitations SET state = ? WHERE invite_id = ?",
            new_status as &InvitationStatus,
            invitation_id
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }
}

impl Crud<Invitation, crate::dtos::CreateInvitationDTO, i32> for InvitationRepository {
    async fn create(&self, data: &crate::dtos::CreateInvitationDTO) -> Result<Invitation, Error> {
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

    async fn update(&self, item: &Invitation) -> Result<Invitation, Error> {
        sqlx::query!(
            r#"
            UPDATE invitations 
            SET target_chat_id = ?, invited_id = ?, invitee_id = ?, state = ?, created_at = ?
            WHERE invite_id = ?
            "#,
            item.target_chat_id,
            item.invited_id,
            item.invitee_id,
            item.state,
            item.created_at,
            item.invite_id
        )
        .execute(&self.connection_pool)
        .await?;

        // Return the updated invitation
        Ok(item.clone())
    }

    async fn delete(&self, id: &i32) -> Result<(), Error> {
        sqlx::query!("DELETE FROM invitations WHERE invite_id = ?", id)
            .execute(&self.connection_pool)
            .await?;

        Ok(())
    }
}
