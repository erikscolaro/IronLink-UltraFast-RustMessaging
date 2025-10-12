//! User DTOs - Data Transfer Objects per utenti

use crate::entities::User;
use serde::{Deserialize, Serialize};

// struct per gestire io col client
#[derive(Serialize, Deserialize, Debug)]
pub struct UserDTO {
    pub id: Option<i32>,
    pub username: Option<String>,
    #[serde(skip_serializing)]
    pub password: Option<String>,
}

impl From<User> for UserDTO {
    fn from(value: User) -> Self {
        Self {
            id: Some(value.user_id),
            username: Some(value.username),
            password: None, // mai esposta al client!!!
        }
    }
}

/// DTO per creare un nuovo utente (senza user_id)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateUserDTO {
    pub username: String,
    pub password: String,
}
