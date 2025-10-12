//! Trait comuni per le repository
//! 
//! Definisce interfacce generiche per operazioni CRUD e funzionalità aggiuntive

/// Trait per operazioni CRUD generiche
/// 
/// # Type Parameters
/// * `T` - Tipo dell'entità gestita dalla repository (es. User, Message, Chat)
/// * `CreateDTO` - DTO per creazione senza ID autogenerato (es. CreateUserDTO)
/// * `Id` - Tipo della chiave primaria (può essere i32, (i32, i32), etc.)
/// 
/// # Nota sul CreateDTO Pattern
/// Il pattern CreateDTO risolve il problema degli ID autogenerati:
/// - Le entities hanno ID (generato dal DB)
/// - I CreateDTO non hanno ID (verrà assegnato dal DB)
/// - create() accetta CreateDTO e restituisce l'entity completa con ID
pub trait Crud<T, CreateDTO, Id> {
    /// Crea un nuovo record usando CreateDTO (senza ID) e restituisce l'entità completa con ID
    async fn create(&self, data: &CreateDTO) -> Result<T, sqlx::Error>;

    /// Legge un record tramite ID
    async fn read(&self, id: &Id) -> Result<Option<T>, sqlx::Error>;

    /// Aggiorna un record esistente
    async fn update(&self, item: &T) -> Result<T, sqlx::Error>;

    /// Cancella un record tramite ID
    async fn delete(&self, id: &Id) -> Result<(), sqlx::Error>;
}

/// Trait per repository che supportano ricerca testuale
pub trait Searchable<T> {
    /// Cerca entità che matchano una query testuale
    async fn search(&self, query: &str) -> Result<Vec<T>, sqlx::Error>;
}

/// Trait per repository con supporto paginazione
pub trait Pageable<T> {
    /// Lista entità con paginazione
    async fn list_paginated(
        &self, 
        limit: i64, 
        offset: i64
    ) -> Result<Vec<T>, sqlx::Error>;
}
