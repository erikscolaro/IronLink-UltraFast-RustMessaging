use crate::{
    dtos::WsEventDTO,
    entities::{IdType, User},
    AppState
};
use axum::extract::ws::Utf8Bytes;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State
    },
    response::Response,
    Extension
};
use futures_util::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt
};
use std::sync::Arc;
use tokio::{
    sync::mpsc::{channel, Receiver},
    time::{sleep, Duration}
};
use crate::dtos::{InvitationDTO, MessageDTO};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<User>, // ottenuto dall'autenticazione JWT
) -> Response {
    let user_id = current_user.user_id;

    // Gestisce automaticamente l'upgrade a WebSocket.
    // Se l'upgrade fallisce, ritorna un errore; altrimenti restituisce la nuova connessione al client.
    ws
        // Possibile limitazione dei buffer, default 128 KB
        //.read_buffer_size(4*1024)
        //.write_buffer_size(16*1024)
        .on_upgrade(move |socket| handle_socket(socket, state, user_id))
}

async fn handle_socket(ws: WebSocket, state: Arc<AppState>, user_id: IdType) {
    // Dividiamo il WebSocket in due metà: sender e receiver
    let (sender, receiver) = ws.split();

    // Creiamo un canale MPSC per comunicazione interna
    // La dimensione qui è logica, non fisica rispetto al WebSocket
    let (tx, rx) = channel(16 * 1024);

    // Salviamo nello stato il trasmettitore associato all'utente
    // Il ricevitore sarà usato dal task dedicato alla scrittura
    state.users_online.insert(user_id, tx);

    // Creiamo due task separati: uno per inviare e uno per ricevere messaggi
    tokio::spawn(write_on_ws(sender, rx, state.clone()));
    tokio::spawn(read_from_ws(receiver, user_id, state.clone()));
}

pub async fn read_from_ws(mut receiver: SplitStream<WebSocket>, user_id: IdType, state: Arc<AppState>) {
    // ciclo su ws per ottenere i messaggi
    while let Some(msg_result) = receiver.next().await {
        // match per vedere se ci sono errori sul ws
        let msg = match msg_result {
            Ok(m) => m,
            Err(err) => {
                if cfg!(debug_assertions) {
                    eprintln!("Errore WebSocket: {}", err);
                }
                break; // esce dal loop ma esegue cleanup dopo
            }
        };

        // Match per tipo di frame ws, se text, close, ping pong ... 
        match msg {
            Message::Text(text) => {
                // match per gestire la deserializzazione e inoltrare ai giusti handler
                match serde_json::from_str::<WsEventDTO>(&text) {
                    Ok(event) => match event {
                        WsEventDTO::Message(msg) => { process_chat_message(state.clone(), user_id, msg).await}
                        WsEventDTO::Invitation(inv) => { process_invitation(state.clone(), user_id, inv).await }
                        _ => {} // non ricevo mai errori o messaggi di sistema dal client
                    },
                    Err(err) => {
                        if cfg!(debug_assertions) {
                            eprintln!("Errore deserializzazione JSON: {}", err);
                        }
                    }
                }
            }
            Message::Close(_) => {
                if cfg!(debug_assertions) {
                    println!("Connessione chiusa dal client");
                }
                break; // esce dal loop per rimuovere l'utente
            }
            _ => {}
        }
    }

    // cleanup finale: rimuove utente dalla lista online
    state.users_online.remove(&user_id);
    if cfg!(debug_assertions) {
        println!("Utente {} rimosso dagli online", user_id);
    }
}

pub async fn write_on_ws(
    mut sender: SplitSink<WebSocket, Message>,
    mut rx: Receiver<WsEventDTO>,
    state: Arc<AppState>,
) {
    // intervallo tra un invio e l'altro per fare batching
    let send_interval = Duration::from_millis(1000);

    // legge rx per ottenere un evento nel formato che vogliamo
    while let Some(event) = rx.recv().await {
        // serializza in json
        match serde_json::to_string(&event) {
            Ok(text) => {
                // serializza di nuovo in utf8bytes, richiesto dal framework
                if sender.send(Message::Text(Utf8Bytes::from(text))).await.is_err() {
                    if cfg!(debug_assertions) {
                        println!("Client disconnesso durante la scrittura");
                    }
                    return;
                }
            }
            Err(err) => {
                if cfg!(debug_assertions) {
                    eprintln!("Errore serializzazione WsEventDTO: {}", err);
                }
            }
        }

        // intervallo tra un invio e l'altro
        sleep(send_interval).await;
    }

    if cfg!(debug_assertions) {
        println!("Task write_on_ws terminato per un utente");
    }
}

pub async fn send_error_to_user(state: &AppState, user_id: IdType, error_code: u16, message: String) {
    if let Some(tx) = state.users_online.get(&user_id) {
        // invio direttamente il WsEventDTO sul canale
        if tx.send(WsEventDTO::Error {code: error_code, message }).await.is_err() {
            if cfg!(debug_assertions) {
                eprintln!("Client disconnesso, errore non inviato all'utente {}", user_id);
            }
        }
    }
}

async fn process_chat_message(state: Arc<AppState>, user_id: IdType, event: MessageDTO){
    todo!()
}

async fn process_invitation(state: Arc<AppState>, user_id: IdType, event: InvitationDTO){
    todo!()
}



