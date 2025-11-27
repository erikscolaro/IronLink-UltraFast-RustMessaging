//! Invitation DTOs - Data Transfer Objects per inviti

use crate::{dtos::{ChatDTO, UserDTO}, entities::{Invitation, InvitationStatus}};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Struct per gestire io col client
#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)] // Non attualmente usata, mantenuta per compatibilità futura
pub struct InvitationDTO {
    pub invite_id: Option<i32>,
    pub target_chat_id: Option<i32>,
    pub invited_id: Option<i32>,
    pub invitee_id: Option<i32>,
    pub state: Option<InvitationStatus>,
    pub created_at: Option<DateTime<Utc>>,
}

impl From<Invitation> for InvitationDTO {
    fn from(value: Invitation) -> Self {
        Self {
            invite_id: Some(value.invite_id),
            target_chat_id: Some(value.target_chat_id),
            invited_id: Some(value.invited_id),
            invitee_id: Some(value.invitee_id),
            state: Some(value.state),
            created_at: Some(value.created_at),
        }
    }
}

/// DTO per creare un nuovo invito (senza invite_id, state e created_at)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateInvitationDTO {
    pub target_chat_id: i32,
    pub invited_id: i32,
    pub invitee_id: i32,
}

/// DTO per aggiornare un invito (solo lo stato è modificabile)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateInvitationDTO {
    pub state: Option<InvitationStatus>,
}

/// DTO arricchito con informazioni complete dell'inviter e della chat
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrichedInvitationDTO {
    pub invite_id: i32,
    pub state: InvitationStatus,
    pub created_at: DateTime<Utc>,
    pub inviter: Option<UserDTO>,
    pub chat: Option<ChatDTO>,
}
