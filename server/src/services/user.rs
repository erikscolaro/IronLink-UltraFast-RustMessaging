//! User services - Gestione utenti

use crate::core::{AppError, AppState};
use crate::dtos::{UserDTO, UserSearchQuery};
use crate::entities::{User, UserRole};
use crate::repositories::{Delete, Read};
use axum::{
    Extension,
    extract::{Json, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use futures::future;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

#[instrument(skip(state), fields(search = %params.search))]
pub async fn search_user_with_username(
    State(state): State<Arc<AppState>>,
    Query(params): Query<UserSearchQuery>, // query params /users/find?search=username
) -> Result<Json<Vec<UserDTO>>, AppError> {
    debug!("Searching users with partial username");
    // 1. Estrarre il parametro search dalla query string
    // 2. Cercare nel database tutti gli utenti con username che contiene parzialmente la query, cercando solo all'inizio dello username
    // 3. Convertire ogni utente trovato in UserDTO
    // 4. Ritornare la lista di UserDTO come risposta JSON
    let users = state
        .user
        .search_by_username_partial(&(params.search))
        .await?;
    info!("Found {} users matching search criteria", users.len());
    let users_dto = users.into_iter().map(UserDTO::from).collect::<Vec<_>>();
    Ok(Json::from(users_dto))
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn get_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i32>, // parametro dalla URL /users/:user_id
) -> Result<Json<Option<UserDTO>>, AppError> {
    debug!("Fetching user by ID");
    // 1. Estrarre user_id dal path della URL
    // 2. Cercare l'utente nel database tramite user_id
    // 3. Se l'utente esiste, convertirlo in UserDTO
    // 4. Ritornare Option<UserDTO> come risposta JSON (Some se trovato, None se non trovato)
    let user_option = state.user.read(&user_id).await?;
    if user_option.is_some() {
        info!("User found");
    } else {
        warn!("User not found");
    }
    Ok(Json(user_option.map(UserDTO::from)))
}

#[instrument(skip(state, current_user), fields(user_id = %current_user.user_id, username = %current_user.username))]
pub async fn delete_my_account(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<impl IntoResponse, AppError> {
    info!("User account deletion initiated");
    // 1. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 2. Recuperare tutti i metadata dell'utente per identificare chat ownership (singola query)
    let user_metadata = state
        .meta
        .find_many_by_user_id(&current_user.user_id)
        .await?;

    debug!("Found {} chat memberships for user", user_metadata.len());

    // 3. Gestire il caso degli ownership: se l'utente è owner di gruppi
    for metadata in &user_metadata {
        if matches!(metadata.user_role, Some(UserRole::Owner)) {
            debug!("Handling ownership transfer for chat {}", metadata.chat_id);
            // Recuperare tutti i membri della chat
            let chat_members = state.meta.find_many_by_chat_id(&metadata.chat_id).await?;

            if chat_members.len() == 1 {
                // Se l'owner è l'unico membro, cancellare la chat completamente
                // ON DELETE CASCADE cancellerà automaticamente i metadata e i messaggi
                info!(
                    "Deleting chat {} (user is the only member)",
                    metadata.chat_id
                );
                state.chat.delete(&metadata.chat_id).await?;
            } else {
                // Cercare un admin a cui trasferire l'ownership
                let new_owner = chat_members
                    .iter()
                    .find(|m| {
                        m.user_id != current_user.user_id
                            && matches!(m.user_role, Some(UserRole::Admin))
                    })
                    .or_else(|| {
                        // Se non c'è un admin, prendi qualsiasi altro membro
                        chat_members
                            .iter()
                            .find(|m| m.user_id != current_user.user_id)
                    });

                if let Some(new_owner) = new_owner {
                    // Trasferire l'ownership
                    info!(
                        "Transferring ownership of chat {} to user {}",
                        metadata.chat_id, new_owner.user_id
                    );
                    state
                        .meta
                        .transfer_ownership(
                            &current_user.user_id,
                            &new_owner.user_id,
                            &metadata.chat_id,
                        )
                        .await?;
                }
            }
        }
    }

    // 4. Cancellare tutti i metadata (UserChatMetadata) associati all'utente
    // (solo per le chat non eliminate al punto 3 - quelle erano già cancellate da CASCADE)
    // Raccogliere le chiavi per la cancellazione

    let meta_ids: Vec<(i32, i32)> = state
        .meta
        .find_many_by_user_id(&current_user.user_id)
        .await?
        .into_iter()
        .map(|m| (m.user_id, m.chat_id))
        .collect();

    debug!("Deleting {} metadata entries", meta_ids.len());
    // Cancellazione effettiva
    future::join_all(meta_ids.iter().map(|k| state.meta.delete(&k))).await;

    // 5-6. Rinominare lo username dell'utente con "Deleted User" e sostituire la password con stringa vuota
    info!("Soft deleting user account");
    state.user.delete(&current_user.user_id).await?;

    // 7-8. Creare un cookie con Max-Age=0 per forzare il logout lato client
    let cookie = "token=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0";
    let mut headers = HeaderMap::new();
    headers.insert("Set-Cookie", HeaderValue::from_str(cookie).unwrap());

    // 9. Ritornare StatusCode::OK con gli headers e messaggio
    info!("Account deleted successfully");
    Ok((StatusCode::OK, headers, "Account deleted successfully"))
}
