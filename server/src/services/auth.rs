//! Auth services - Gestione autenticazione e registrazione utenti

use crate::core::{encode_jwt, AppError, AppState};
use crate::dtos::UserDTO;
use crate::entities::User;
use crate::repositories::Crud;
use axum::{
    extract::{Json, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;

pub async fn login_user(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UserDTO>, // JSON body
) -> Result<impl IntoResponse, AppError> {
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
    let username = body.username.ok_or(AppError::with_message(
        StatusCode::BAD_REQUEST,
        "Invalid username or password",
    ))?;

    let user = match state.user.find_by_username(&username).await? {
        Some(user) => user,
        None => return Err(AppError::with_status(StatusCode::UNAUTHORIZED)),
    };

    if let Some(password) = &body.password {
        if !user.verify_password(password) {
            return Err(AppError::with_message(
                StatusCode::UNAUTHORIZED,
                "Username or password are not correct.",
            ));
        }
    } else {
        return Err(AppError::with_message(
            StatusCode::UNAUTHORIZED,
            "Password was not provided.",
        ));
    }

    let token = encode_jwt(user.username, user.user_id, &state.jwt_secret)?;

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

    Ok((StatusCode::OK, headers))
}

pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UserDTO>, // JSON body
) -> Result<Json<UserDTO>, AppError> {
    // 1. Verificare che lo username sia presente nel body, altrimenti ritornare errore BAD_REQUEST
    // 2. Verificare che la password sia presente nel body e abbia almeno 8 caratteri, altrimenti ritornare errore BAD_REQUEST
    // 3. Bloccare il caso in cui l'username è "Deleted User" (controllo string prima della query DB)
    // 4. Generare l'hash della password fornita (prima della query DB per parallelizzare il lavoro CPU)
    // 5. Se la generazione dell'hash fallisce, ritornare errore INTERNAL_SERVER_ERROR
    // 6. Controllare se esiste già un utente con lo stesso username nel database
    // 7. Se l'utente esiste già, ritornare errore CONFLICT con messaggio "Username already exists"
    // 8. Creare un nuovo oggetto CreateUserDTO con username e password hashata
    // 9. Salvare il nuovo utente nel database tramite il metodo create, fornendo l'oggetto CreateUserDTO
    // 10. Convertire l'utente creato ritornato dal metodo in UserDTO
    // 11. Ritornare il DTO dell'utente creato come risposta JSON
    let username = if let Some(username) = &body.username {
        username.clone()
    } else {
        return Err(AppError::with_message(
            StatusCode::BAD_REQUEST,
            "Username is required"
        ));
    };
    let password = if let Some(password) = &body.password {
        password.clone()
    } else {
        return Err(AppError::with_message(
            StatusCode::BAD_REQUEST, 
            "Password is required"
        ));
    };
    if state.user.find_by_username(&username).await.is_ok() {
        return Err(AppError::with_message(
            StatusCode::CONFLICT, 
            "Username already exists"
        ));
    }
    let password_hash = if let Ok(hash) = User::hash_password(&password) {
        hash
    } else {
        return Err(AppError::with_message(
            StatusCode::INTERNAL_SERVER_ERROR, 
            "Failed to hash password"
        ));
    };
    let new_user = crate::dtos::CreateUserDTO {
        username,
        password: password_hash,
    };

    let created_user = state.user.create(&new_user).await?;
    
    Ok(Json(UserDTO::from(created_user)))
}
