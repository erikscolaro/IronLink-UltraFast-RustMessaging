//! User entity - Entità utente con metodi per gestione password

use bcrypt::{DEFAULT_COST, hash, verify};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    /* se vogliamo rinominare campi usiamo la macro
     * #[serde(rename = "userId")]
     */
    pub user_id: i32,
    pub username: String,
    pub password: String,
}

impl User {
    /// Verify if target_password matches the stored hashed password
    pub fn verify_password(&self, target_password: &String) -> bool {
        verify(target_password, &self.password).unwrap_or(false)
    }

    /// Hash a password using bcrypt with default cost
    pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
        let hash = hash(password, DEFAULT_COST)?;
        Ok(hash)
    }
}
