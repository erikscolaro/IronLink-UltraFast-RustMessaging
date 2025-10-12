//! WebSocket Connection Management - Gestione connessioni WebSocket

use crate::AppState;
use crate::dtos::WsEventDTO;
use crate::ws::event_handlers::{process_chat_message, process_invitation};
use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use futures_util::{
    SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use std::sync::Arc;
use tokio::{
    sync::mpsc::{Receiver, channel},
    time::{Duration, sleep},
};

/// Gestisce la connessione WebSocket dopo l'upgrade
/// Operazioni:
/// 1. Dividere il WebSocket in sender e receiver
/// 2. Creare un canale MPSC per comunicazione interna
/// 3. Registrare l'utente nella mappa users_online
/// 4. Avviare due task separati per lettura e scrittura
pub async fn handle_socket(ws: WebSocket, state: Arc<AppState>, user_id: i32) {
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

/// Legge messaggi dal WebSocket del client
/// Operazioni:
/// 1. Ricevere messaggi dal WebSocket
/// 2. Gestire frame Close, Text, Ping/Pong
/// 3. Deserializzare eventi JSON (WsEventDTO)
/// 4. Inoltrare agli handler appropriati
/// 5. Cleanup: rimuovere utente da users_online alla disconnessione
pub async fn read_from_ws(
    mut receiver: SplitStream<WebSocket>,
    user_id: i32,
    state: Arc<AppState>,
) {
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
                        WsEventDTO::Message(msg) => {
                            process_chat_message(state.clone(), user_id, msg).await
                        }
                        WsEventDTO::Invitation(inv) => {
                            process_invitation(state.clone(), user_id, inv).await
                        }
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

/// Scrive messaggi sul WebSocket verso il client
/// Operazioni:
/// 1. Ricevere eventi dal canale MPSC interno
/// 2. Serializzare WsEventDTO in JSON
/// 3. Inviare il messaggio sul WebSocket
/// 4. Applicare rate limiting (1 secondo tra invii)
pub async fn write_on_ws(
    mut sender: SplitSink<WebSocket, Message>,
    mut rx: Receiver<WsEventDTO>,
    _state: Arc<AppState>,
) {
    // intervallo tra un invio e l'altro per fare batching
    let send_interval = Duration::from_millis(1000);

    // legge rx per ottenere un evento nel formato che vogliamo
    while let Some(event) = rx.recv().await {
        // serializza in json
        match serde_json::to_string(&event) {
            Ok(text) => {
                // serializza di nuovo in utf8bytes, richiesto dal framework
                if sender
                    .send(Message::Text(Utf8Bytes::from(text)))
                    .await
                    .is_err()
                {
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
