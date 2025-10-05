use crate::auth::encode_jwt;
use crate::dtos::{ChatDTO, MessageDTO, SearchQueryDTO, UserDTO, UserInChatDTO};
use crate::entities::{Chat, ChatType, User, UserChatMetadata, UserRole};
use crate::error_handler::AppError;
use crate::repositories::Crud;
use crate::AppState;
use axum::{
    extract::{Json, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension,
};
use axum_macros::debug_handler;
use futures_util::future::try_join_all;
use serde_json::json;
use std::sync::Arc;
use chrono::Utc;
/* si usano queste ntoazioni per prendere cioò che ci serve dai frame http, mi raccomando
  funziona solo in questo esatto ordine ! Json consuma tutto il messaggio quindi deve stare ultimo
        State(state): State<Arc<AppState>>,
        Path(user_id): Path<i32>,        // parametro dalla URL /users/:user_id
        Path(chat_id): Path<i32>,        // parametro dalla URL /users/:user_id
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
    Json(body): Json<UserDTO>, // JSON body
) -> Result<impl IntoResponse, AppError> {
    // cerco l'utente, se  non lo trovo allora errore
    let username = body.username.ok_or(AppError::with_message(
        StatusCode::BAD_REQUEST,
        "Invalid username or password",
    ))?;

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
    
    // Verifica che username sia fornito 
    let username = if let Some(username) = &body.username {
        username.clone()
    } else {
        return Err(AppError::with_message(
            StatusCode::BAD_REQUEST,
            "Username is required"
        ));
    };
    // Verifica che password sia fornita 
    let password = if let Some(password) = &body.password {
        password.clone()
    } else {
        return Err(AppError::with_message(
            StatusCode::BAD_REQUEST, 
            "Password is required"
        ));
    };
    // Verifica che l'utente non esista già
    if state.user.find_by_username(&username).await.is_ok() {
        return Err(AppError::with_message(
            StatusCode::CONFLICT, 
            "Username already exists"
        ));
    }
    // Hash della password 
    let password_hash = if let Ok(hash) = User::hash_password(&password) {
        hash        //se l'operazione va a buon fine, restituisco l'hash
    } else {
        return Err(AppError::with_message(
            StatusCode::INTERNAL_SERVER_ERROR, 
            "Failed to hash password"
        ));
    };
    // Crea il nuovo utente
    let new_user = User {
        user_id: 0, 
        username,
        password: password_hash,
    };

    // Salva nel database e ritorna il DTO
    let created_user = state.user.create(&new_user).await?;
    
    Ok(Json(UserDTO::from(created_user)))
}

pub async fn search_user_with_username(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQueryDTO>, // query params /users/find?search=username
) -> Result<Json<Vec<UserDTO>>, AppError> {
    let query = params.search.filter(|v| v.len() >= 3).ok_or_else(|| {
        AppError::with_message(StatusCode::BAD_REQUEST, "Query search param too short.")
    })?;
    let users = state.user.search_by_username_partial(&query).await?;
    let users_dto = users.into_iter().map(UserDTO::from).collect::<Vec<_>>();
    Ok(Json::from(users_dto))
}
pub async fn get_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i32>, // parametro dalla URL /users/:user_id
) -> Result<Json<Option<UserDTO>>, AppError> {
    let user_option = state.user.read(&user_id).await?;
    Ok(Json(user_option.map(UserDTO::from)))
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
    Extension(current_user): Extension<User>,
) -> Result<Json<Vec<ChatDTO>>, AppError> {
    // prendi tutti i chat_id dell'utente
    let chat_ids: Vec<i32> = state
        .meta
        .find_all_by_user_id(&current_user.user_id)
        .await?
        .into_iter()
        .map(|s| s.chat_id)
        .collect();

    let chats: Vec<Chat> = try_join_all(chat_ids.into_iter().map(|cid| {
        let state = state.clone(); // se AppState è Arc, clonalo
        async move { state.chat.read(&cid).await }
    }))
    .await?
    .into_iter()
    .filter_map(|c| c) // prende solo i Some
    .collect();

    // converti in DTO
    let chats_dto: Vec<ChatDTO> = chats.into_iter().map(ChatDTO::from).collect();

    Ok(Json(chats_dto))
}

#[debug_handler]
pub async fn create_chat(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
    Json(body): Json<ChatDTO>,
) -> Result<Json<ChatDTO>, AppError> {
    // ricordarsi che la crazione di una chat privata non ha titolo o descrizione, usare il flag chat type

    // !! WIP 

    let chat;
    match body.chat_type {
    //se la chat è privata, controllare che non esista già una chat privata tra i due utenti
        ChatType::Private =>{
            
            // Verifica che ci siano esattamente 2 utenti
            let user_list = body.user_list.as_ref().ok_or_else(|| AppError::with_message(
                StatusCode::BAD_REQUEST,
                "Private chat should specify user list.",
            ))?;
            
            if user_list.len() != 2 {
                return Err(AppError::with_message(
                    StatusCode::BAD_REQUEST,
                    "Private chat should specify exactly two users.",
                ));
            }
            
            //cerca id del secondo utente
            let second_user_id = user_list.iter()
                .find(|&&id| id != current_user.user_id)
                .ok_or_else(|| AppError::with_message(
                    StatusCode::BAD_REQUEST,
                    "Current user must be one of the two users.",
                ))?;
            
            // se esiste, cerca chat privata tra i due
            let existing_chat = state.chat.get_private_chat_between_users(&current_user.user_id, second_user_id).await?;
            // se esiste, errore
            if existing_chat.is_some() {
                return Err(AppError::with_message(
                    StatusCode::CONFLICT,
                    "A private chat between these users already exists.",
                ));
            }
            // se non esiste, crea la chat privata con i due utenti
            let new_chat = Chat {
                chat_id: 0, // will be set by database
                title: None,
                description: None,
                chat_type: ChatType::Private,
            };
            chat = state.chat.create(&new_chat).await?;
            // crea i metadata per entrambi gli utenti, entrambi standard

            let metadata_current_user = UserChatMetadata {
                user_id: current_user.user_id,
                chat_id: chat.chat_id,
                user_role: Some(UserRole::Standard),
                member_since: Utc::now(),
                messages_visible_from: Utc::now(),
                messages_received_until: Utc::now(),
            };

            let metadata_second_user = UserChatMetadata {
                user_id: second_user_id.clone(),
                chat_id: chat.chat_id,
                user_role: Some(UserRole::Standard),
                member_since: Utc::now(),
                messages_visible_from: Utc::now(),
                messages_received_until: Utc::now(),
            };
            //inserisci i metadata specificati nel database con create
            state.meta.create(&metadata_current_user).await?;
            state.meta.create(&metadata_second_user).await?;
        }

    //se la chat è di gruppo, allora current user diventa owner automaticamente
        ChatType::Group => {
           // crea la chat di gruppo
            let new_chat = Chat {
                chat_id: 0, // will be set by database
                title: body.title.clone(),
                description: body.description.clone(),
                chat_type: ChatType::Group,
            };
            chat = state.chat.create(&new_chat).await?;

            // crea i metadata per l'owner
            let metadata_owner = UserChatMetadata {
                user_id: current_user.user_id,
                chat_id: chat.chat_id,
                user_role: Some(UserRole::Owner),
                member_since: Utc::now(),
                messages_visible_from: Utc::now(),
                messages_received_until: Utc::now(),
            };

            //inserisci i metadata specificati nel database con create
            state.meta.create(&metadata_owner).await?;
        }
    }

    // ritorna la chat creata come DTO
    let chat_dto = ChatDTO::from(chat);
    Ok(Json(chat_dto))
    
}
pub async fn get_chat_messages(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    // Query(params): Query<QueryStructCursom>,   // da vedere, se conviene o meno
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<Json<Vec<MessageDTO>>, AppError> {
    todo!()
}
pub async fn list_chat_members(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
) -> Result<Json<Vec<UserInChatDTO>>, AppError> {
    todo!()
}

pub async fn invite_to_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Path(user_id): Path<i32>,
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
    Path(user_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
    Json(body): Json<UserRole>,
) -> Result<(), AppError> {
    //ricordare di controllare i permessi per fare questo , forse conviene creare un middleware?

    // inviare messaggio di aggiornamento ruolo sul gruppo
    todo!()
}
pub async fn transfer_ownership(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    //anche qui, devo controlalre se current user è owner per trasferire l'ownership
    // devo aggionrare i metadata di entrambi
    // devo inviare un messaggio di sistema per informare nel gruppo

    todo!()
}
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Path(user_id): Path<i32>,
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
    Path(chat_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // solo cancellare il metadata
    // inviare messaggio di sistema

    todo!()
}
