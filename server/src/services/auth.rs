//! Auth services - Gestione autenticazione e registrazione utenti

use crate::core::{AppError, AppState, encode_jwt};
use crate::dtos::{CreateUserDTO, UserDTO};
use crate::entities::User;
use crate::repositories::Create;
use axum::{
    extract::{Json, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};
use validator::Validate;

/// DTO per il login (solo username e password)
#[derive(serde::Deserialize)]
pub struct LoginDTO {
    pub username: String,
    pub password: String,
}

#[instrument(skip(state, body), fields(username = %body.username))]
pub async fn login_user(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginDTO>, // JSON body
) -> Result<impl IntoResponse, AppError> {
    debug!("Login attempt for user");
    // 1. Estrarre lo username dal body della richiesta, ritornare errore BAD_REQUEST se mancante
    // 2. Verificare che la password sia stata fornita nel body, altrimenti ritornare errore UNAUTHORIZED (fail-fast prima della query DB)
    // 3. Bloccare il caso in cui si sta cercando di fare login con "Deleted User" (controllo string prima della query DB)
    // 4. Cercare l'utente nel database tramite username
    // 5. Se l'utente non esiste, ritornare errore UNAUTHORIZED
    // 6. Verificare che la password fornita, dopo essere hashata, corrisponda all'hash memorizzato
    // 7. Se la password non corrisponde, ritornare errore UNAUTHORIZED con messaggio specifico
    // 8. Generare un token JWT con il metodo encode che prende in input userid, username e il segreto
    // 9. Costruire un cookie HttpOnly, Secure, SameSite=Lax con il token e durata 24 ore
    // 10. Creare gli headers HTTP con Set-Cookie e Authorization (Bearer token)
    // 11. Ritornare StatusCode::OK con gli headers

    if body.username == "Deleted User" {
        warn!("Login attempt with 'Deleted User' username");
        return Err(AppError::unauthorized("Invalid username or password"));
    }

    let user = match state.user.find_by_username(&body.username).await? {
        Some(user) => {
            debug!("User found in database");
            user
        }
        None => {
            warn!("User not found in database");
            return Err(AppError::unauthorized("Invalid username or password"));
        }
    };

    if !user.verify_password(&body.password) {
        warn!("Invalid password for user");
        return Err(AppError::unauthorized(
            "Username or password are not correct.",
        ));
    }

    let token = encode_jwt(&user.username, user.user_id, &state.jwt_secret)?;

    let cookie_value = format!(
        "token={}; HttpOnly; Secure; SameSite=Lax; Max-Age={}",
        token,
        24 * 60 * 60
    );

    let mut headers = HeaderMap::new();
    headers.insert("Set-Cookie", HeaderValue::from_str(&cookie_value).unwrap());
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
    );

    info!("User logged in successfully: {}", user.username);
    Ok((StatusCode::OK, headers))
}

#[instrument(skip(state, body), fields(username = %body.username))]
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateUserDTO>, // JSON body
) -> Result<Json<UserDTO>, AppError> {
    debug!("User registration attempt");
    // 1. Validare il DTO con validator (username/password format, lunghezza, "Deleted User")
    // 2. Controllare se esiste già un utente con lo stesso username nel database
    // 3. Se l'utente esiste già, ritornare errore CONFLICT con messaggio "Username already exists"
    // 4. Generare l'hash della password fornita
    // 5. Se la generazione dell'hash fallisce, ritornare errore INTERNAL_SERVER_ERROR
    // 6. Creare un nuovo oggetto CreateUserDTO con username e password hashata
    // 7. Salvare il nuovo utente nel database tramite il metodo create
    // 8. Convertire l'utente creato in UserDTO
    // 9. Ritornare il DTO dell'utente creato come risposta JSON

    // Validazione con validator (include controllo "Deleted User")
    body.validate()?;

    // Controllare se esiste già un utente con lo stesso username
    if let Some(_) = state.user.find_by_username(&body.username).await? {
        warn!("Username already exists");
        return Err(AppError::conflict("Username already exists"));
    }

    let password_hash = User::hash_password(&body.password).map_err(|e| {
        error!("Failed to hash password: {:?}", e);
        AppError::internal_server_error("Failed to hash password")
    })?;

    let new_user = CreateUserDTO {
        username: body.username.clone(),
        password: password_hash,
    };

    let created_user = state.user.create(&new_user).await?;

    info!("User registered successfully: {}", created_user.username);
    Ok(Json(UserDTO::from(created_user)))
}
