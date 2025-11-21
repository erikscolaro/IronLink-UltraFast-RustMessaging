//! Query DTOs - Data Transfer Objects per query di ricerca

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// DTO per query parameters di ricerca utenti
#[derive(Serialize, Deserialize, Debug)]
pub struct UserSearchQuery {
    pub search: String,
}

/// DTO per query parameters di paginazione messaggi
#[derive(Serialize, Deserialize, Debug)]
pub struct MessagesQuery {
    #[serde(default)]
    pub before_date: Option<DateTime<Utc>>,
}
