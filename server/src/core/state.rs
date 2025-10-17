//! Application State - Stato globale dell'applicazione
//!
//! Contiene tutti i repository, configurazioni e stato condiviso
//! necessario per gestire l'applicazione.

use crate::repositories::{
    ChatRepository, InvitationRepository, MessageRepository, UserChatMetadataRepository,
    UserRepository,
};
use crate::ws::chatmap::ChatMap;
use crate::ws::usermap::UserMap;
use sqlx::MySqlPool;

/// Stato globale dell'applicazione condiviso tra tutte le route e middleware
pub struct AppState {
    /// Repository per la gestione degli utenti
    pub user: UserRepository,

    /// Repository per la gestione delle chat
    pub chat: ChatRepository,

    /// Repository per la gestione dei messaggi
    pub msg: MessageRepository,

    /// Repository per la gestione degli inviti
    pub invitation: InvitationRepository,

    /// Repository per la gestione dei metadati utente-chat
    pub meta: UserChatMetadataRepository,

    /// Secret key per JWT token
    pub jwt_secret: String,

    /// Mappa concorrente degli utenti online con i loro canali WebSocket
    /// Key: user_id, Value: Sender per inviare messaggi al WebSocket dell'utente
    pub users_online: UserMap,

    /// Struttura di gestione delle chat con almeno un utente online
    pub chats_online: ChatMap,
}

impl AppState {
    /// Crea una nuova istanza di AppState inizializzando tutti i repository
    /// con il pool di connessioni fornito e la JWT secret.
    ///
    /// # Arguments
    /// * `pool` - Pool di connessioni MySQL condiviso
    /// * `jwt_secret` - Chiave segreta per la firma dei token JWT
    ///
    /// # Returns
    /// Nuova istanza di AppState con tutti i repository inizializzati
    pub fn new(pool: MySqlPool, jwt_secret: String) -> Self {
        Self {
            user: UserRepository::new(pool.clone()),
            chat: ChatRepository::new(pool.clone()),
            msg: MessageRepository::new(pool.clone()),
            invitation: InvitationRepository::new(pool.clone()),
            meta: UserChatMetadataRepository::new(pool),
            jwt_secret,
            users_online: UserMap::new(),
            chats_online: ChatMap::new(),
        }
    }
}
