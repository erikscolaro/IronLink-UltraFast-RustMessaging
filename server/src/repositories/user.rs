//! UserRepository - Repository per la gestione degli utenti

use super::{Create, Delete, Read, Update};
use crate::dtos::{CreateUserDTO, UpdateUserDTO};
use crate::entities::User;
use sqlx::{Error, MySqlPool};
use tracing::{debug, info, instrument, warn};

// USER REPO
pub struct UserRepository {
    connection_pool: MySqlPool,
}

impl UserRepository {
    pub fn new(connection_pool: MySqlPool) -> UserRepository {
        Self { connection_pool }
    }

    ///considero l'username univoco
    /// Find user by exact username match
    /// For partial username search, use search_by_username_partial
    #[instrument(skip(self), fields(username = %username))]
    pub async fn find_by_username(&self, username: &String) -> Result<Option<User>, Error> {
        debug!("Finding user by username");
        let user = sqlx::query_as!(
            User,
            "SELECT user_id, username, password FROM users WHERE username = ?",
            username
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        if user.is_some() {
            info!("User found");
        } else {
            debug!("User not found");
        }

        Ok(user)
    }

    /// Search users by partial username match (for search functionality)
    #[instrument(skip(self), fields(pattern = %username_pattern))]
    pub async fn search_by_username_partial(
        &self,
        username_pattern: &String,
    ) -> Result<Vec<User>, Error> {
        debug!("Searching users with partial username match");
        let pattern = format!("{}%", username_pattern);
        let users = sqlx::query_as!(
            User,
            "SELECT user_id, username, password FROM users WHERE username LIKE ? LIMIT 10",
            pattern
        )
        .fetch_all(&self.connection_pool)
        .await?;

        info!("Found {} users matching pattern", users.len());
        Ok(users)
    }
}

impl Create<User, CreateUserDTO> for UserRepository {
    #[instrument(skip(self, data), fields(username = %data.username))]
    async fn create(&self, data: &CreateUserDTO) -> Result<User, Error> {
        debug!("Creating new user");
        // Insert user and get the ID using MySQL syntax
        let result = sqlx::query!(
            "INSERT INTO users (username, password) VALUES (?, ?)",
            data.username,
            data.password
        )
        .execute(&self.connection_pool)
        .await?;

        // Get the last inserted ID
        let new_id = result.last_insert_id() as i32;

        info!("User created with id {}", new_id);

        // Return the created user with the new ID
        Ok(User {
            user_id: new_id,
            username: data.username.clone(),
            password: data.password.clone(),
        })
    }
}

impl Read<User, i32> for UserRepository {
    #[instrument(skip(self), fields(user_id = %id))]
    async fn read(&self, id: &i32) -> Result<Option<User>, Error> {
        debug!("Reading user by id");
        let user = sqlx::query_as!(
            User,
            "SELECT user_id, username, password FROM users WHERE user_id = ?",
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        if user.is_some() {
            debug!("User found");
        } else {
            debug!("User not found");
        }

        Ok(user)
    }
}

impl Update<User, UpdateUserDTO, i32> for UserRepository {
    #[instrument(skip(self, data), fields(user_id = %id))]
    async fn update(&self, id: &i32, data: &UpdateUserDTO) -> Result<User, Error> {
        debug!("Updating user");
        // First, get the current user to ensure it exists
        let current_user = self
            .read(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // Only password can be updated
        if let Some(ref password) = data.password {
            debug!("Updating user password");
            sqlx::query!(
                "UPDATE users SET password = ? WHERE user_id = ?",
                password,
                id
            )
            .execute(&self.connection_pool)
            .await?;

            info!("User password updated");

            // Fetch and return the updated user
            self.read(id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
        } else {
            // If no password provided, return current user unchanged
            debug!("No password update provided, returning current user");
            Ok(current_user)
        }
    }
}

impl Delete<i32> for UserRepository {
    /// Soft delete user by setting username to "Deleted User" and clearing password ""
    /// This preserves message history while anonymizing the user
    #[instrument(skip(self), fields(user_id = %user_id))]
    async fn delete(&self, user_id: &i32) -> Result<(), Error> {
        debug!("Soft deleting user");
        sqlx::query!(
            "UPDATE users SET username = 'Deleted User', password = '' WHERE user_id = ?",
            user_id
        )
        .execute(&self.connection_pool)
        .await?;

        info!("User soft deleted successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use sqlx::MySqlPool;

    /// Test generico - esempio di utilizzo di #[sqlx::test]
    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_example(_pool: MySqlPool) -> sqlx::Result<()> {
        // Il database è stato creato automaticamente con migrations applicate
        // I fixtures users.sql sono stati caricati
        // Implementa qui i tuoi test per UserRepository
        Ok(())
    }
}
