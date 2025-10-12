//! Invitation entity - Entità invito

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::enums::InvitationStatus;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Invitation {
    pub invite_id: i32, //todo: valutare se togliere la chaive primaria qui, gli inviti sono univoci per invitee e chat_id
    // rinominato da group_id a chat_id per consistenza
    pub target_chat_id: i32, // chat ( di gruppo ) in cui si viene invitati
    pub invited_id: i32,     // utente invitato
    pub invitee_id: i32,     // utente che invita
    pub state: InvitationStatus,
    pub created_at: DateTime<Utc>,
}
