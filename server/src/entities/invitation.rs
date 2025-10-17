//! Invitation entity - Entità invito

use super::enums::InvitationStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Invitation {
    pub invite_id: i32,
    pub target_chat_id: i32, // chat ( di gruppo ) in cui si viene invitati
    pub invited_id: i32,     // utente invitato
    pub invitee_id: i32,     // utente che invita
    pub state: InvitationStatus,
    pub created_at: DateTime<Utc>,
}
