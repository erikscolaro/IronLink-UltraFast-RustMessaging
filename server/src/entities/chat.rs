//! Chat entity - Entità chat

use serde::{Deserialize, Serialize};
use super::enums::ChatType;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chat {
    pub chat_id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub chat_type: ChatType,
}
