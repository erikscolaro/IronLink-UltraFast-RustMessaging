//! Query DTOs - Data Transfer Objects per query di ricerca

use serde::{Deserialize, Serialize};

/// DTO per ricerche con query string
#[derive(Serialize, Deserialize, Debug)]
pub struct SearchQueryDTO {
    pub search: Option<String>,
}
