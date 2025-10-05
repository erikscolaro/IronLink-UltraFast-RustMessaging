use crate::AppState;
use crate::auth::encode_jwt;
use crate::dtos::{ChatDTO, MessageDTO, SearchQueryDTO, UserDTO, UserInChatDTO};
use crate::entities::{Chat, IdType, User, UserRole};
use crate::error_handler::AppError;
use crate::repositories::Crud;
use axum::{
    Extension,
    extract::{Json, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response}
};
use axum_macros::debug_handler;
use serde_json::json;
use std::sync::Arc;
/* si usano queste ntoazioni per prendere cioò che ci serve dai frame http, mi raccomando
  funziona solo in questo esatto ordine ! Json consuma tutto il messaggio quindi deve stare ultimo
        State(state): State<Arc<AppState>>,
        Path(user_id): Path<IdType>,        // parametro dalla URL /users/:user_id
        Path(chat_id): Path<IdType>,        // parametro dalla URL /users/:user_id
        Query(params): Query<QueryStructCursom>,   // query params ?filter=xyz
        Extension(current_user): Extension<User> // ottenuto dall'autenticazione tramite token jwt
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
    Json(body): Json<UserDTO>,   // JSON body
) -> Result<impl IntoResponse, AppError> {
    // cerco l'utente, se  non lo trovo allora errore
    let username = body.username.ok_or(AppError::with_message(StatusCode::BAD_REQUEST, "Invalid username or password"))?;
    
    let user = match state.user.find_by_username(&username).await? {
        Some(user) => user,
        None => return Err(AppError::with_status(StatusCode::UNAUTHORIZED)),
    };

    // Verify the password provided against the stored hash
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

    // Costruisci cookie con direttive di sicurezza
    let cookie_value = format!(
        "token={}; HttpOnly; Secure; SameSite=Lax; Max-Age={}",
        token,
        24 * 60 * 60 // durata in secondi
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
    todo!()
}

pub async fn search_user_with_username(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQueryDTO>, // query params /users/find?search=username
) -> Result<Json<Vec<UserDTO>>, AppError> {
    let query = params.search
        .filter(|v| v.len() >= 3)
        .ok_or_else(|| AppError::with_message(StatusCode::BAD_REQUEST, "Query search param too short."))?;
    let users = state.user.search_by_username_partial(&query).await?;
    let users_dto = users.into_iter().map(UserDTO::from).collect::<Vec<_>>();
    Ok(Json::from(users_dto))
}
pub async fn get_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<IdType>, // parametro dalla URL /users/:user_id
) -> Result<Json<Option<UserDTO>>, AppError> {
    todo!()
}

pub async fn delete_my_account(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<impl IntoResponse, AppError> {
    // logica di cancellazione account
    // occhio, la cancellazione rinomina username con Deleted User e sostituisce la password con ""
    // altrimenti ci tocca modificare nella tabella messaggi tutti i messaggi dell'utente!
    // anche i descrittori metdata vanno cancellati, ma questo è inevitabile
    // lato client, bisogna gestire i messaggi inviati da utenti cancellati in questo modo:
    // se l'id è nella lista dei membri della chat, allora mostra il nome
    // altrimenti, mostra deleted user

    // anche il caso in cui un utente owner di un gruppo si cancella va gestito
    // cosa succede? si cancellano tutti i gruppi di sua appartenenza ? oppure si identifica randomicamente
    // una persona tra gli admin che prende il possesso del gruppo ?

    // sovrascrive il cookie lato client con uno che scade subito, per forzare il logout
    let cookie = "token=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0";
    let mut headers = HeaderMap::new();
    headers.insert("Set-Cookie", HeaderValue::from_str(cookie).unwrap());
    Ok((StatusCode::OK, headers, "Logged out"))
}
pub async fn list_chats(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<Json<Vec<ChatDTO>>, AppError> {
    todo!()
}

#[debug_handler]
pub async fn create_chat(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
    Json(body): Json<ChatDTO>,
) -> Result<Json<ChatDTO>, AppError> {
    // ricordarsi che la crazione di una chat privata non ha titolo o descrizione, usare il flag chat type
    todo!()
}
pub async fn get_chat_messages(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<IdType>,
    // Query(params): Query<QueryStructCursom>,   // da vedere, se conviene o meno
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<Json<Vec<MessageDTO>>, AppError> {
    todo!()
}
pub async fn list_chat_members(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<IdType>,
) -> Result<Json<Vec<UserInChatDTO>>, AppError> {
    todo!()
}

pub async fn invite_to_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<IdType>,
    Path(user_id): Path<IdType>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // possiamo dare più significato alla questione dei ruoli se solo admin o owner possono invitare persone

    // crea una chat privata con l'utente target se non esiste già e lo informa della craezione della chat
    // in questa chat privata, attraverso il canale websocket, ricavabile dallo stato (todo) e salva in dabaase persistnete
    // invia alla persona un emssaggio di sistema con l'invito
    // bisogna garantire l'ubnicità dell'invito, uindi prima fare un check per vedere se ci sono inviti pending
    todo!()
}

#[debug_handler]
pub async fn update_member_role(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<IdType>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
    Json(body): Json<UserRole>,
) -> Result<(), AppError> {
    //ricordare di controllare i permessi per fare questo , forse conviene creare un middleware?

    // inviare messaggio di aggiornamento ruolo sul gruppo
    todo!()
}
pub async fn transfer_ownership(    
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<IdType>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    //anche qui, devo controlalre se current user è owner per trasferire l'ownership
    // devo aggionrare i metadata di entrambi
    // devo inviare un messaggio di sistema per informare nel gruppo

    todo!()
}
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<IdType>,
    Path(user_id): Path<IdType>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // controllare se sono admin o owner
    // cancellare i metadata
    // come gestiamo i messaggi delle persone uscite? li lasciamo dove stanno? direi di si
    // ricordarsi di inviare messaggio di sistema
    todo!()
}
pub async fn leave_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<IdType>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // solo cancellare il metadata
    // inviare messaggio di sistema

    todo!()
}

