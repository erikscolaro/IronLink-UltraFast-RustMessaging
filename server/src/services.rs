use crate::AppState;
use crate::auth::encode_jwt;
use crate::error_handler::AppError;
use crate::models::User;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::Arc;
/* si usano queste ntoazioni per prendere cioò che ci serve dai frame http, mi raccomando
  funziona solo in questo esatto ordine !
   State(state): State<Arc<AppState>>,
   Path(user_id): Path<i32>,        // parametro dalla URL /users/:user_id
   Query(params): Query<QueryStructCursom>,   // query params ?filter=xyz
   Json(body): Json<MyBody>,        // JSON body

   per query (ovvero ciò chge segue il ? come ?search=...&last=... ) si usa creare dei models appositi
   in questo modo, possiamo utilizzare le macro serde per fare la validazione dei dati
   stesso discorso per json :D
   todo: creare più models se necessario ,ad esempio userPublic o groupCreate
   nota: ci sarebbe anche extension per ottenere l'utnete direttamente dal middleware di autenticazione
*/

/*
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct MyStruct {
    #[serde(rename = "user_id")]       // cambia il nome del campo in JSON
    id: i32,

    #[serde(skip_serializing_if = "Option::is_none")] // salta se None
    nickname: Option<String>,

    #[serde(default)]                  // usa default se manca nel JSON
    active: bool,

    #[serde(flatten)]                  // fonde un'altra struct dentro
    extra: ExtraData,
}

#[derive(Serialize, Deserialize)]
struct ExtraData {
    country: String,
    city: String,
}

 */

pub async fn root() -> Response {
    Json(json!({
        "status": "ok",
        "message": "Server is running"
    }))
    .into_response()
}

// Nota: possiamo usare tranquillametne l'operatore ? nel caso in cui ci siano degli errori dal momento che
// la AppError implementa il metodo from E quindi in automatico fa il cast di un generico errore a AppError
// e visto che AppError implementa il tratto into_response, questo viene automaticamente convertito a una rispsota
// valida per il client con una struttura predeterminata.

pub async fn login_user(
    State(state): State<Arc<AppState>>,
    // Path(user_id): Path<i32>,        // parametro dalla URL /users/:user_id
    // Query(params): Query<MyQuery>,   // query params ?filter=xyz
    Json(body): Json<User>, // JSON body
) -> Result<impl IntoResponse, AppError> {
    // cerco l'utente, se  non lo trovo allora
    let user = match state.user.find_by_username(&body.username).await? {
        Some(user) => user,
        None => return Err(AppError::from_status(StatusCode::UNAUTHORIZED)),
    };

    // Verify the password provided against the stored hash
    if !user.verify_password(&body.password) {
        return Err(AppError::from_status(StatusCode::UNAUTHORIZED)); // Password verification failed, return unauthorized status
    }

    let token = encode_jwt(user.username, user.id.expect("ahhahaha"), &state.jwt_secret)?;

    // Return the token as a JSON-wrapped string
    Ok(Json(token))
}

pub async fn logout_user() -> Result<(), AppError> {
    todo!()
}

pub async fn register_user() -> Result<impl IntoResponse, AppError> {
    todo!()
}

pub async fn search_users() -> Result<impl IntoResponse, AppError> {
    todo!()
}
pub async fn get_user() -> Result<impl IntoResponse, AppError> {
    todo!()
}
pub async fn delete_my_account() -> Result<(), AppError> {
    todo!()
}
pub async fn list_chats() -> Result<impl IntoResponse, AppError> {
    todo!()
}
pub async fn create_chat() -> Result<impl IntoResponse, AppError> {
    todo!()
}
pub async fn get_chat_messages() -> Result<impl IntoResponse, AppError> {
    todo!()
}
pub async fn list_chat_members() -> Result<impl IntoResponse, AppError> {
    todo!()
}
pub async fn invite_to_chat() -> Result<(), AppError> {
    todo!()
}
pub async fn update_member_role() -> Result<(), AppError> {
    todo!()
}
pub async fn transfer_ownership() -> Result<(), AppError> {
    todo!()
}
pub async fn remove_member() -> Result<(), AppError> {
    todo!()
}
pub async fn leave_chat() -> Result<(), AppError> {
    todo!()
}
