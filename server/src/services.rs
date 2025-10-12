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
    // 1. Creare una risposta JSON con stato "ok" e messaggio "Server is running"
    // 2. Convertire la risposta in un tipo Response valido
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
    // 8. Creare un nuovo oggetto UserDTO con username e password hashata
    // 9. Salvare il nuovo utente nel database tramite il metodo create, fornendo l'oggetto UserDTO
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
    let new_user = UserDTO {
        id: None,
        username: Some(username),
        password: Some(password_hash),
    };

    let created_user = state.user.create(&new_user).await?;
    
    Ok(Json(UserDTO::from(created_user)))
}

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
    // 3. Gestire il caso degli ownership: se l'utente è owner di gruppi, trasferire l'ownership a un admin casuale se esiste, altrimenti a una persona a caso
    // 4. Cancellare tutti i metadata (UserChatMetadata) associati all'utente
    // 5. Rinominare lo username dell'utente con "Deleted User" 
    // 6. Sostituire la password dell'utente con stringa vuota
    // 7. Creare un cookie con Max-Age=0 per forzare il logout lato client
    // 8. Inserire il cookie negli headers HTTP con Set-Cookie
    // 9. Ritornare StatusCode::OK con gli headers e messaggio "Logged out"
    // Nota: i messaggi dell'utente rimangono nel database ma lato client vanno mostrati come "Deleted User"
    let cookie = "token=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0";
    let mut headers = HeaderMap::new();
    headers.insert("Set-Cookie", HeaderValue::from_str(cookie).unwrap());
    Ok((StatusCode::OK, headers, "Logged out"))
}

pub async fn list_chats(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>,
) -> Result<Json<Vec<ChatDTO>>, AppError> {
    // 1. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 2. Recuperare tutti i metadata dell'utente dal database tramite user_id (singola query, da implementare)
    // 3. Estrarre tutti i chat_id dai metadata trovati
    // 4. Recuperare tutte le chat in una singola query batch (WHERE chat_id IN (...)) invece di query multiple. chiamarlo find_multiple
    // 5. Convertire ogni Chat in ChatDTO (trasformazione in memoria, nessun I/O)
    // 6. Ritornare la lista di ChatDTO come risposta JSON
    let chat_ids: Vec<i32> = state
        .meta
        .find_all_by_user_id(&current_user.user_id)
        .await?
        .into_iter()
        .map(|s| s.chat_id)
        .collect();

    let chats: Vec<Chat> = try_join_all(chat_ids.into_iter().map(|cid| {
        let state = state.clone();
        async move { state.chat.read(&cid).await }
    }))
    .await?
    .into_iter()
    .filter_map(|c| c)
    .collect();

    let chats_dto: Vec<ChatDTO> = chats.into_iter().map(ChatDTO::from).collect();

    Ok(Json(chats_dto))
}

#[debug_handler]
pub async fn create_chat(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
    Json(body): Json<ChatDTO>,
) -> Result<Json<ChatDTO>, AppError> {
    // CASO ChatType::Private:
    // 1. Verificare che user_list sia presente nel body, altrimenti errore BAD_REQUEST
    // 2. Verificare che user_list contenga esattamente 2 utenti, altrimenti errore BAD_REQUEST
    // 3. Verificare che current_user sia uno dei due utenti, altrimenti errore BAD_REQUEST
    // 4. Identificare l'user_id del secondo utente (diverso da current_user)
    // 5. Cercare se esiste già una chat privata tra i due utenti (query DB solo dopo validazioni)
    // 6. Se esiste già, ritornare errore CONFLICT
    // 7. Creare ChatCreateDTO con title=None, description=None, chat_type=Private
    // 8. Salvare la chat nel database (la chiave primaria è autoincrementale)
    // 9. Creare metadata per entrambi gli utenti con ruolo Standard e timestamp correnti (preparazione in memoria)
    // 10. Salvare entrambi i metadata nel database in batch/transazione
    // 
    // CASO ChatType::Group:
    // 1. Creare ChatCreateDTO con title e description dal body, chat_type=Group
    // 2. Salvare la chat nel database (la chiave primaria è autoincrementale)
    // 3. Creare metadata per current_user con ruolo Owner e timestamp correnti
    // 4. Salvare il metadata nel database
    // 
    // FINALE:
    // 1. Convertire la chat creata in ChatDTO (trasformazione in memoria)
    // 2. Ritornare il ChatDTO come risposta JSON
    let chat;
    match body.chat_type {
        ChatType::Private =>{
            
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
            
            let second_user_id = user_list.iter()
                .find(|&&id| id != current_user.user_id)
                .ok_or_else(|| AppError::with_message(
                    StatusCode::BAD_REQUEST,
                    "Current user must be one of the two users.",
                ))?;
            
            let existing_chat = state.chat.get_private_chat_between_users(&current_user.user_id, second_user_id).await?;
            if existing_chat.is_some() {
                return Err(AppError::with_message(
                    StatusCode::CONFLICT,
                    "A private chat between these users already exists.",
                ));
            }
            let new_chat = Chat {
                chat_id: 0,
                title: None,
                description: None,
                chat_type: ChatType::Private,
            };
            chat = state.chat.create(&new_chat).await?;

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
            state.meta.create(&metadata_current_user).await?;
            state.meta.create(&metadata_second_user).await?;
        }

        ChatType::Group => {
            let new_chat = Chat {
                chat_id: 0,
                title: body.title.clone(),
                description: body.description.clone(),
                chat_type: ChatType::Group,
            };
            chat = state.chat.create(&new_chat).await?;

            let metadata_owner = UserChatMetadata {
                user_id: current_user.user_id,
                chat_id: chat.chat_id,
                user_role: Some(UserRole::Owner),
                member_since: Utc::now(),
                messages_visible_from: Utc::now(),
                messages_received_until: Utc::now(),
            };

            state.meta.create(&metadata_owner).await?;
        }
    }

    let chat_dto = ChatDTO::from(chat);
    Ok(Json(chat_dto))
    
}
pub async fn get_chat_messages(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    // Query(params): Query<QueryStructCursom>,   // da vedere, se conviene o meno
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<Json<Vec<MessageDTO>>, AppError> {
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare il metadata dell'utente per questa chat (singola query che fa sia controllo membership che recupero timestamp)
    // 4. Se metadata non esiste (utente non membro), ritornare errore FORBIDDEN
    // 5. Recuperare tutti i messaggi della chat filtrati per timestamp >= messages_visible_from in una singola query
    // 6. Convertire ogni messaggio in MessageDTO (trasformazione in memoria, nessun I/O)
    // 7. Ritornare la lista di MessageDTO come risposta JSON
    // Nota: paginazione (limit, offset) da implementare in futuro tramite Query params
    todo!()
}
pub async fn list_chat_members(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<Json<Vec<UserInChatDTO>>, AppError> {
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare tutti i metadata associati alla chat tramite chat_id (singola query)
    // 4. Verificare se current_user è tra i membri, altrimenti ritornare errore FORBIDDEN (controllo in memoria)
    // 5. Estrarre tutti gli user_id dai metadata
    // 6. Recuperare tutti gli utenti in una singola query batch (WHERE user_id IN (...))
    // 7. Combinare le informazioni degli utenti con i metadata (join in memoria)
    // 8. Convertire ogni combinazione in UserInChatDTO (trasformazione in memoria)
    // 9. Ritornare la lista di UserInChatDTO come risposta JSON
    todo!()
}

pub async fn invite_to_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Path(user_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // 1. Estrarre chat_id e user_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare il metadata di current_user per questa chat (singola query per controllo permessi)
    // 4. Verificare che current_user sia Admin o Owner, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 5. Verificare che l'utente target non sia già membro della chat (query metadata target)
    // 6. Se è già membro, ritornare errore CONFLICT
    // 7. Controllare se esiste già un invito pending per questo utente in questa chat
    // 8. Se esiste già un invito pending, ritornare errore CONFLICT
    // 9. Verificare che l'utente target esista nel database (query solo se tutte le validazioni passano)
    // 10. Se non esiste, ritornare errore NOT_FOUND
    // 11. Creare o recuperare una chat privata tra current_user e l'utente target
    // 12. Creare un messaggio di sistema con l'invito alla chat
    // 13. Salvare il messaggio di invito nel database
    // 14. Inviare il messaggio tramite WebSocket all'utente target se online (operazione non bloccante)
    // 15. Ritornare StatusCode::OK
    todo!()
}

#[debug_handler]
pub async fn update_member_role(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
    Json(body): Json<UserRole>,
) -> Result<(), AppError> {
    // 1. Estrarre user_id dal path della URL e nuovo ruolo dal body JSON
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare il chat_id dal contesto (mancante nel path, da aggiungere alla signature)
    // 4. Recuperare in parallelo i metadata di current_user e target_user per questa chat (implementare una nuova query nel repo find_multiple con WHERE IN)
    // 5. Verificare che entrambi siano membri della chat, altrimenti ritornare errore appropriato
    // 6. Verificare che current_user sia Admin o Owner, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 7. Verificare le regole di promozione: Owner può modificare tutti, Admin può modificare solo Standard (controllo in memoria)
    // 8. Se le regole non sono rispettate, ritornare errore FORBIDDEN
    // 9. Aggiornare il campo user_role nei metadata dell'utente target
    // 10. Creare un messaggio di sistema che notifica il cambio di ruolo
    // 11. Salvare il messaggio nel database
    // 12. Inviare il messaggio tramite WebSocket a tutti i membri online (operazione non bloccante)
    // 13. Ritornare StatusCode::OK
    todo!()
}
pub async fn transfer_ownership(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare l'user_id del nuovo owner dal body della richiesta (mancante nella signature, da aggiungere)
    // 4. Recuperare in parallelo i metadata di current_user e nuovo_owner per questa chat (2 query parallele o singola WHERE IN)
    // 5. Verificare che current_user sia Owner della chat, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 6. Verificare che il nuovo owner sia membro della chat, altrimenti ritornare errore BAD_REQUEST
    // 7. Aggiornare i metadata di entrambi gli utenti in transazione: current_user diventa Admin, nuovo_owner diventa Owner
    // 8. Creare un messaggio di sistema che notifica il trasferimento di ownership
    // 9. Salvare il messaggio nel database
    // 10. Inviare il messaggio tramite WebSocket a tutti i membri online (operazione non bloccante)
    // 11. Ritornare StatusCode::OK
    todo!()
}
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Path(user_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // 1. Estrarre chat_id e user_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare in parallelo i metadata di current_user e target_user per questa chat (implementare una nuova query nel repo find_multiple con WHERE IN)
    // 4. Verificare che current_user sia Admin o Owner, altrimenti ritornare errore FORBIDDEN (fail-fast)
    // 5. Verificare che l'utente target sia membro della chat, altrimenti ritornare errore NOT_FOUND
    // 6. Verificare che non si stia cercando di rimuovere l'Owner, altrimenti ritornare errore FORBIDDEN (controllo in memoria)
    // 7. Cancellare i metadata dell'utente target per questa chat dal database
    // 8. Creare un messaggio di sistema che notifica la rimozione del membro (i messaggi dell'utente rimangono nel DB)
    // 9. Salvare il messaggio nel database
    // 10. Inviare il messaggio tramite WebSocket a tutti i membri online incluso il rimosso (operazione non bloccante)
    // 11. Ritornare StatusCode::OK
    todo!()
}
pub async fn leave_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i32>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione tramite token jwt
) -> Result<(), AppError> {
    // 1. Estrarre chat_id dal path della URL
    // 2. Ottenere l'utente corrente dall'Extension (autenticato tramite JWT)
    // 3. Recuperare il metadata di current_user per questa chat (singola query)
    // 4. Se metadata non esiste (non membro), ritornare errore NOT_FOUND (fail-fast)
    // 5. Verificare il ruolo: se è Owner, ritornare errore CONFLICT con messaggio specifico (fail-fast, controllo in memoria)
    // 6. Cancellare i metadata di current_user per questa chat dal database
    // 7. Creare un messaggio di sistema che notifica l'uscita (i messaggi dell'utente rimangono nel DB)
    // 8. Salvare il messaggio nel database
    // 9. Inviare il messaggio tramite WebSocket a tutti i membri online (operazione non bloccante)
    // 10. Ritornare StatusCode::OK
    todo!()
}
