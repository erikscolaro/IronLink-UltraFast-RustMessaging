//! User services - Gestione utenti

use crate::core::{AppError, AppState};
use crate::dtos::{SearchQueryDTO, UserDTO};
use crate::entities::{User, UserRole};
use crate::repositories::{Delete, Read};
use axum::{
    Extension,
    extract::{Json, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;

pub async fn search_user_with_username(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQueryDTO>, // query params /users/find?search=username
) -> Result<Json<Vec<UserDTO>>, AppError> {
    // 1. Estrarre il parametro search dalla query string
    // 2. Verificare che la lunghezza della stringa di ricerca sia almeno 3 caratteri
    // 3. Se troppo corta, ritornare errore BAD_REQUEST con messaggio "Query search param too short."
    // 4. Cercare nel database tutti gli utenti con username che contiene parzialmente la query, cercando solo all'inizio dello username
    // 5. Convertire ogni utente trovato in UserDTO
    // 6. Ritornare la lista di UserDTO come risposta JSON
    let query = params
        .search
        .filter(|v| v.len() >= 3)
        .ok_or_else(|| AppError::bad_request("Query search param too short."))?;
    let users = state.user.search_by_username_partial(&query).await?;
    let users_dto = users.into_iter().map(UserDTO::from).collect::<Vec<_>>();
    Ok(Json::from(users_dto))
}

pub async fn get_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i32>, // parametro dalla URL /users/:user_id
) -> Result<Json<Option<UserDTO>>, AppError> {
    // 1. Estrarre user_id dal path della URL
    // 2. Cercare l'utente nel database tramite user_id
    // 3. Se l'utente esiste, convertirlo in UserDTO
    // 4. Ritornare Option<UserDTO> come risposta JSON (Some se trovato, None se non trovato)
    let user_option = state.user.read(&user_id).await?;
    Ok(Json(user_option.map(UserDTO::from)))
}

pub async fn delete_my_account(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<impl IntoResponse, AppError> {
    // 1. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 2. Recuperare tutti i metadata dell'utente per identificare chat ownership (singola query)
    let user_metadata = state
        .meta
        .find_many_by_user_id(&current_user.user_id)
        .await?;

    // 3. Gestire il caso degli ownership: se l'utente è owner di gruppi
    for metadata in &user_metadata {
        if matches!(metadata.user_role, Some(UserRole::Owner)) {
            // Recuperare tutti i membri della chat
            let chat_members = state.meta.find_many_by_chat_id(&metadata.chat_id).await?;

            if chat_members.len() == 1 {
                // Se l'owner è l'unico membro, cancellare la chat completamente
                // ON DELETE CASCADE cancellerà automaticamente i metadata e i messaggi
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
    state.meta.delete(&current_user.user_id).await?;

    // 5-6. Rinominare lo username dell'utente con "Deleted User" e sostituire la password con stringa vuota
    state.user.delete(&current_user.user_id).await?;

    // 7-8. Creare un cookie con Max-Age=0 per forzare il logout lato client
    let cookie = "token=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0";
    let mut headers = HeaderMap::new();
    headers.insert("Set-Cookie", HeaderValue::from_str(cookie).unwrap());

    // 9. Ritornare StatusCode::OK con gli headers e messaggio
    Ok((StatusCode::OK, headers, "Account deleted successfully"))
}
