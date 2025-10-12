//! UserRepository - Repository per la gestione degli utenti

use super::Crud;
use crate::entities::User;
use sqlx::{Error, MySqlPool};

//MOD -> possibile modifica
// Controllare se in alcuni casi non vogliamo l'oggetto come risultato ma solo un valore, e viceversa
//Per le crud, non sempre ritorno l'oggetto, quindi servirà poi fare una lettura successiva in services oppure scriverlo nel messaggio ok()
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
    pub async fn find_by_username(&self, username: &String) -> Result<Option<User>, Error> {
        let user = sqlx::query_as!(
            User,
            "SELECT user_id, username, password FROM users WHERE username = ?",
            username
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(user)
    }

    /// Search users by partial username match (for search functionality)
    pub async fn search_by_username_partial(
        &self,
        username_pattern: &String,
    ) -> Result<Vec<User>, Error> {
        let pattern = format!("{}%", username_pattern);
        let users = sqlx::query_as!(
            User,
            "SELECT user_id, username, password FROM users WHERE username LIKE ? LIMIT 10",
            pattern
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(users)
    }
}

impl Crud<User, crate::dtos::CreateUserDTO, crate::dtos::UpdateUserDTO, i32> for UserRepository {
    async fn create(&self, data: &crate::dtos::CreateUserDTO) -> Result<User, Error> {
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

        // Return the created user with the new ID
        Ok(User {
            user_id: new_id,
            username: data.username.clone(),
            password: data.password.clone(),
        })
    }

    async fn read(&self, id: &i32) -> Result<Option<User>, Error> {
        let user = sqlx::query_as!(
            User,
            "SELECT user_id, username, password FROM users WHERE user_id = ?",
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(user)
    }

    async fn update(&self, id: &i32, data: &crate::dtos::UpdateUserDTO) -> Result<User, Error> {
        // First, get the current user to ensure it exists
        let current_user = self
            .read(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // Only password can be updated
        if let Some(ref password) = data.password {
            sqlx::query!(
                "UPDATE users SET password = ? WHERE user_id = ?",
                password,
                id
            )
            .execute(&self.connection_pool)
            .await?;

            // Fetch and return the updated user
            self.read(id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
        } else {
            // If no password provided, return current user unchanged
            Ok(current_user)
        }
    }

    /// Soft delete user by setting username to "Deleted User" and clearing password ""
    /// This preserves message history while anonymizing the user
    async fn delete(&self, user_id: &i32) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE users SET username = 'Deleted User', password = '' WHERE user_id = ?",
            user_id
        )
        .execute(&self.connection_pool)
        .await?;

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
