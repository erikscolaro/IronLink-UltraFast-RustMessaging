//! WebSocket Connection Management - Gestione connessioni WebSocket

use crate::{
    AppState,
    dtos::WsEventDTO,
    ws::event_handlers::{process_chat_message, process_invitation},
};
use axum::extract::ws::Utf8Bytes;
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::Duration;
use tokio::time::{interval, timeout};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{StreamMap};
use crate::ws::{BATCH_INTERVAL, BATCH_MAX_SIZE, RATE_LIMITER_MILLIS, TIMEOUT_DURATION_SECONDS};

pub enum InternalSignal {
    Shutdown,
    AddChat(i32),
    RemoveChat(i32),
}



pub async fn handle_socket(ws: WebSocket, state: Arc<AppState>, user_id: i32) {
    // Dividiamo il WebSocket in due metà: sender e receiver
    let (ws_tx, ws_rx) = ws.split();

    // Creiamo un canale unbounded per comunicazione interna
    // considerare passaggio a unbounded channel per non perdere eventuali segnali
    let (int_tx, int_rx) = unbounded_channel::<InternalSignal>();

    // Salviamo nello stato il trasmettitore di watch associato all'utente
    // Il ricevitore sarà usato dal task dedicato alla scrittura
    state.users_online.insert(user_id, int_tx.clone());

    // dobbiamo iniziare un task che stia in ascolto del websocket
    tokio::spawn(listen_ws(user_id, ws_rx, int_tx.clone(), state.clone()));

    // creare un task che sta in ascolto sull'insieme dei canali broadcast
    tokio::spawn(write_ws(user_id, ws_tx, int_rx, state));
}

pub async fn write_ws(
    user_id: i32,
    mut websocket_tx: SplitSink<WebSocket, Message>,
    mut internal_rx: UnboundedReceiver<InternalSignal>,
    state: Arc<AppState>,
) {
    let chat_vec: Vec<i32> = match state.meta.find_many_by_user_id(&user_id).await {
        Ok(chats) => chats.iter().map(|m| m.chat_id).collect(),
        Err(_) => return, // Termina se DB fallisce
    };

    let mut stream_map = StreamMap::new();

    state
        .chats_online
        .subscribe_multiple(chat_vec.clone())
        .into_iter()
        .zip(chat_vec.iter())
        .for_each(|(rx, &chat_id)| {
            stream_map.insert(chat_id, BroadcastStream::new(rx));
        });

    let mut batch = Vec::new();
    let mut interval = tokio::time::interval(Duration::from_millis(BATCH_INTERVAL));
    interval.tick().await; // Consuma primo tick immediato

    loop {
        tokio::select! {
                    Some((chat_id, result)) = tokio_stream::StreamExt::next(&mut stream_map) => {
                        if let Ok(msg) = result {
                            batch.push((chat_id, msg));

                            if batch.len() >= BATCH_MAX_SIZE {
                                if send_batch(&mut websocket_tx, &batch).await.is_err() {
                                    break;
                                }
                                batch.clear();
                            }
                        }
                    }

                    // serve per fare in modo di inviare dei messaggi anche se il batch non è arrivato a 10
                    // altrimenti aspetterei troppo
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            if send_batch(&mut websocket_tx, &batch).await.is_err() {
                                break;
                            }
                            batch.clear();
                        }
                    }

                    signal = internal_rx.recv() => {
                        match signal {
                            Some(InternalSignal::Shutdown) => break,
                            Some(InternalSignal::AddChat(chat_id)) => {
                                let rx = state.chats_online.subscribe(&chat_id);
                                stream_map.insert(chat_id, BroadcastStream::new(rx));
                            }
                            Some(InternalSignal::RemoveChat(chat_id)) => {
                                stream_map.remove(&chat_id);
                            }
                            None => break, // canale chiuso, quindi listener ws chius, quindi stacca tutto
                        }
                    }
                }
    }
    // Invia batch finale prima di terminare
    if !batch.is_empty() {
        let _ = send_batch(&mut websocket_tx, &batch).await;
    }
}

async fn send_batch(
    websocket_tx: &mut SplitSink<WebSocket, Message>,
    batch: &[(i32, Arc<WsEventDTO>)],
) -> Result<(), axum::Error> {
    let events: Vec<_> = batch.iter().map(|(_, event)| event).collect();
    let json = serde_json::to_string(&events)
        .map_err(|e| axum::Error::new(e))?;
    websocket_tx
        .send(Message::Text(Utf8Bytes::from(json)))
        .await
}

pub async fn listen_ws(
    user_id: i32,
    mut websocket_rx: SplitStream<WebSocket>,
    internal_tx: UnboundedSender<InternalSignal>,
    state: Arc<AppState>,
) {
    let mut rate_limiter = interval(Duration::from_millis(RATE_LIMITER_MILLIS));
    let timeout_duration = Duration::from_secs(TIMEOUT_DURATION_SECONDS);

    loop {
        match timeout(timeout_duration, StreamExt::next(&mut websocket_rx)).await {
            Ok(Some(msg_result)) => {
                rate_limiter.tick().await;

                let msg = match msg_result {
                    Ok(m) => m,
                    Err(_) => break,
                };

                match msg {
                    Message::Text(text) => {
                        if let Ok(event) = serde_json::from_str::<WsEventDTO>(&text) {
                            match event {
                                WsEventDTO::Message(msg) => {
                                    process_chat_message(&state, user_id, msg).await
                                }
                                WsEventDTO::Invitation(inv) => {
                                    process_invitation(&state, user_id, inv).await
                                }
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            Ok(None) | Err(_) => break,
        }
    }

    // Cleanup
    let _ = internal_tx.send(InternalSignal::Shutdown);
    state.users_online.remove(&user_id);
}
