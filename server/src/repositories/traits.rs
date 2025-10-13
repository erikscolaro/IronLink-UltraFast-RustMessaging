//! Common repository traits
//!
//! This module defines generic interfaces for database operations.

/// Trait for creating new entities in the database
///
/// # Type Parameters
/// * `Entity` - Type of the returned entity (with ID assigned by the database)
/// * `CreateDTO` - DTO for creation (without ID, will be automatically generated)
pub trait Create<Entity, CreateDTO> {
    /// Creates a new entity in the database
    ///
    /// # Arguments
    /// * `data` - DTO containing the data for creation (without ID)
    ///
    /// # Returns
    /// * `Ok(Entity)` - Created entity with ID assigned by the database
    /// * `Err(sqlx::Error)` - Error during insertion
    async fn create(&self, data: &CreateDTO) -> Result<Entity, sqlx::Error>;
}

/// Trait for reading a single entity by primary key
///
/// # Type Parameters
/// * `Entity` - Type of the entity to read
/// * `Id` - Type of the primary key (e.g. `i32`, `String`, `(i32, i32)`)
pub trait Read<Entity, Id> {
    /// Reads an entity from the database by its primary key
    ///
    /// # Arguments
    /// * `id` - Primary key of the entity to read
    ///
    /// # Returns
    /// * `Ok(Some(Entity))` - Entity found
    /// * `Ok(None)` - No entity with that ID
    /// * `Err(sqlx::Error)` - Error during reading
    async fn read(&self, id: &Id) -> Result<Option<Entity>, sqlx::Error>;
}

/// Trait for reading multiple entities by list of primary keys
///
/// # Type Parameters
/// * `Entity` - Type of the entities to read
/// * `Id` - Type of the primary key (e.g. `i32`, `String`, `(i32, i32)`)
pub trait ReadMany<Entity, Id> {
    /// Reads multiple entities from the database by their primary keys
    ///
    /// # Arguments
    /// * `ids` - Slice of primary keys of the entities to read
    ///
    /// # Returns
    /// * `Ok(Vec<Entity>)` - Vec containing all found entities (can be empty)
    /// * `Err(sqlx::Error)` - Error during reading
    ///
    /// # Note
    /// Entities are returned in the order they are found in the database,
    /// which may not match the order of the provided IDs.
    async fn read_many(&self, ids: &[Id]) -> Result<Vec<Entity>, sqlx::Error>;
}

/// Trait for updating existing entities
///
/// # Type Parameters
/// * `Entity` - Type of the updated entity
/// * `UpdateDTO` - DTO for updating (optional fields for partial updates)
/// * `Id` - Type of the primary key
pub trait Update<Entity, UpdateDTO, Id> {
    /// Updates an existing entity in the database
    ///
    /// # Arguments
    /// * `id` - Primary key of the entity to update
    /// * `data` - DTO containing the fields to update (only `Some(_)` fields are modified)
    ///
    /// # Returns
    /// * `Ok(Entity)` - Updated entity
    /// * `Err(sqlx::Error)` - Error during update (e.g. entity not found)
    async fn update(&self, id: &Id, data: &UpdateDTO) -> Result<Entity, sqlx::Error>;
}

/// Trait for deleting entities
///
/// # Type Parameters
/// * `Id` - Type of the primary key
pub trait Delete<Id> {
    /// Deletes an entity from the database
    ///
    /// # Arguments
    /// * `id` - Primary key of the entity to delete
    ///
    /// # Returns
    /// * `Ok(())` - Deletion successful
    /// * `Err(sqlx::Error)` - Error during deletion
    async fn delete(&self, id: &Id) -> Result<(), sqlx::Error>;
}
