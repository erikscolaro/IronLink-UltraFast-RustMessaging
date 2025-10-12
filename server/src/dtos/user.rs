//! User DTOs - Data Transfer Objects per utenti

use crate::entities::User;
use serde::{Deserialize, Serialize};
use validator::Validate;

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
#[derive(Serialize, Deserialize, Debug, Clone, Validate)]
pub struct CreateUserDTO {
    #[validate(length(min = 3, max = 50, message = "Username must be between 3 and 50 characters"))]
    #[validate(custom(function = "validate_username", message = "Username can only contain letters, numbers, and underscores"))]
    pub username: String,
    
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    #[validate(custom(function = "validate_password_strength", message = "Password must contain at least one uppercase, one lowercase, and one number"))]
    pub password: String,
}

fn validate_username(username: &str) -> Result<(), validator::ValidationError> {
    lazy_static::lazy_static! {
        static ref USERNAME_REGEX: regex::Regex = regex::Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
    }
    
    // Bloccare "Deleted User" - username riservato dal sistema
    if username == "Deleted User" {
        return Err(validator::ValidationError::new("reserved_username"));
    }
    
    if USERNAME_REGEX.is_match(username) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_username"))
    }
}

fn validate_password_strength(password: &str) -> Result<(), validator::ValidationError> {
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    
    if has_uppercase && has_lowercase && has_digit {
        Ok(())
    } else {
        Err(validator::ValidationError::new("weak_password"))
    }
}

/// DTO per aggiornare un utente esistente (solo password modificabile)
#[derive(Serialize, Deserialize, Debug, Clone, Validate)]
pub struct UpdateUserDTO {
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    #[validate(custom(function = "validate_password_strength", message = "Password must contain at least one uppercase, one lowercase, and one number"))]
    pub password: Option<String>,
}
