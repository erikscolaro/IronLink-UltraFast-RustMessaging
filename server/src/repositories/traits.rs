//! Trait comuni per le repository
//!
//! Definisce interfacce generiche per operazioni CRUD e funzionalità aggiuntive

/// Trait per operazioni CRUD generiche
///
/// # Type Parameters
/// * `T` - Tipo dell'entità gestita dalla repository (es. User, Message, Chat)
/// * `CreateDTO` - DTO per creazione senza ID autogenerato (es. CreateUserDTO)
/// * `UpdateDTO` - DTO per aggiornamento parziale con campi opzionali (es. UpdateUserDTO)
/// * `Id` - Tipo della chiave primaria (può essere i32, (i32, i32), etc.)
///
/// # Nota sul DTO Pattern
/// Il pattern DTO risolve diversi problemi:
///
/// **CreateDTO**: Gestisce ID autogenerati
/// - Le entities hanno ID (generato dal DB)
/// - I CreateDTO non hanno ID (verrà assegnato dal DB)
/// - create() accetta CreateDTO e restituisce l'entity completa con ID
///
/// **UpdateDTO**: Gestisce aggiornamenti parziali e sicurezza
/// - Tutti i campi sono Option<T> per permettere update parziali
/// - Solo i campi con Some(_) vengono aggiornati nel DB
/// - Campi immutabili (id, created_at) non sono presenti nell'UpdateDTO
/// - update() richiede ID esplicito + UpdateDTO
pub trait Crud<T, CreateDTO, UpdateDTO, Id> {
    /// Crea un nuovo record usando CreateDTO (senza ID) e restituisce l'entità completa con ID
    async fn create(&self, data: &CreateDTO) -> Result<T, sqlx::Error>;

    /// Legge un record tramite ID
    async fn read(&self, id: &Id) -> Result<Option<T>, sqlx::Error>;

    /// Aggiorna un record esistente usando UpdateDTO (campi parziali)
    async fn update(&self, id: &Id, data: &UpdateDTO) -> Result<T, sqlx::Error>;

    /// Cancella un record tramite ID
    async fn delete(&self, id: &Id) -> Result<(), sqlx::Error>;
}
