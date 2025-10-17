//! WebSocket Connection Management - Gestione connessioni WebSocket

use crate::ws::{BATCH_INTERVAL, BATCH_MAX_SIZE, RATE_LIMITER_MILLIS, TIMEOUT_DURATION_SECONDS};
use crate::{
    AppState,
    dtos::MessageDTO,
    ws::{event_handlers::process_message, usermap::InternalSignal},
};
use axum::extract::ws::Utf8Bytes;
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::{sync::Arc};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::Duration;
use tokio::time::{interval, timeout};
use tokio_stream::StreamMap;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{error, info, instrument, warn};

#[instrument(skip(ws, state), fields(user_id))]
pub async fn handle_socket(ws: WebSocket, state: Arc<AppState>, user_id: i32) {
    info!("WebSocket connection established");

    // Dividiamo il WebSocket in due metà: sender e receiver
    let (ws_tx, ws_rx) = ws.split();

    // Creiamo un canale unbounded per comunicazione interna
    // considerare passaggio a unbounded channel per non perdere eventuali segnali
    let (int_tx, int_rx) = unbounded_channel::<InternalSignal>();

    // Salviamo nello stato il trasmettitore di watch associato all'utente
    // Il ricevitore sarà usato dal task dedicato alla scrittura
    state.users_online.register_online(user_id, int_tx.clone());
    info!("User registered as online");

    // dobbiamo iniziare un task che stia in ascolto del websocket
    tokio::spawn(listen_ws(user_id, ws_rx, int_tx.clone(), state.clone()));

    // creare un task che sta in ascolto sull'insieme dei canali broadcast
    tokio::spawn(write_ws(user_id, ws_tx, int_rx, state));
}

#[instrument(skip(websocket_tx, internal_rx, state), fields(user_id))]
pub async fn write_ws(
    user_id: i32,
    mut websocket_tx: SplitSink<WebSocket, Message>,
    mut internal_rx: UnboundedReceiver<InternalSignal>,
    state: Arc<AppState>,
) {
    info!("Write task started");

    let chat_vec: Vec<i32> = match state.meta.find_many_by_user_id(&user_id).await {
        Ok(chats) => {
            info!(chat_count = chats.len(), "User chats loaded");
            chats.iter().map(|m| m.chat_id).collect()
        }
        Err(e) => {
            error!("Failed to load user chats: {:?}", e);
            return; // Termina se DB fallisce
        }
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

    let mut batch: Vec<Arc<MessageDTO>> = Vec::new();
    let mut interval = tokio::time::interval(Duration::from_millis(BATCH_INTERVAL));
    interval.tick().await; // Consuma primo tick immediato

    'external: loop {
        tokio::select! {
            Some((_, result)) = tokio_stream::StreamExt::next(&mut stream_map) => {
                if let Ok(msg) = result {
                    batch.push(msg);
                    if batch.len() >= BATCH_MAX_SIZE {
                        if send_batch(&mut websocket_tx, &batch).await.is_err() {
                            warn!("Failed to send batch, closing connection");
                            break 'external;
                        }
                        info!(batch_size = batch.len(), "Batch sent");
                        batch.clear();
                    }
                }
            }

            // serve per fare in modo di inviare dei messaggi anche se il batch non è arrivato a 10
            // altrimenti aspetterei troppo
            _ = interval.tick() => {
                if !batch.is_empty() {
                    if send_batch(&mut websocket_tx, &batch).await.is_err() {
                        warn!("Failed to send batch on interval, closing connection");
                        break 'external;
                    }
                    info!(batch_size = batch.len(), "Batch sent on interval");
                    batch.clear();
                }
            }

            signal = internal_rx.recv() => {
                match signal {
                    Some(InternalSignal::Shutdown) => {
                        info!("Shutdown signal received");
                        break 'external;
                    }
                    Some(InternalSignal::AddChat(chat_id)) => {
                        info!(chat_id, "Adding chat subscription");
                        let rx = state.chats_online.subscribe(&chat_id);
                        stream_map.insert(chat_id, BroadcastStream::new(rx));
                    }
                    Some(InternalSignal::RemoveChat(chat_id)) => {
                        info!(chat_id, "Removing chat subscription");
                        stream_map.remove(&chat_id);
                    }
                    Some(InternalSignal::Error(err_msg)) => {
                        warn!(error_message = err_msg, "Sending error message to client");
                        if let Err(e) = websocket_tx.send(Message::Text(Utf8Bytes::from(err_msg))).await {
                            error!("Failed to send error message: {:?}", e);
                            break;
                        }
                    }
                    Some(InternalSignal::Invitation(invitation)) => {
                        info!(invite_id = invitation.invite_id, "Sending invitation to client");
                        if let Ok(json) = serde_json::to_string(&invitation) {
                            if let Err(e) = websocket_tx.send(Message::Text(Utf8Bytes::from(json))).await {
                                error!("Failed to send invitation: {:?}", e);
                                break 'external;
                            }
                        } else {
                            error!("Failed to serialize invitation");
                        }
                    }
                    None => {
                        info!("Internal channel closed");
                        break 'external; // canale chiuso, quindi listener ws chius, quindi stacca tutto
                    }
                }
            }
        }
    }

    // Invia batch finale prima di terminare
    if !batch.is_empty() {
        info!(
            batch_size = batch.len(),
            "Sending final batch before shutdown"
        );
        let _ = send_batch(&mut websocket_tx, &batch).await;
    }

    info!("Write task terminated");
}

#[instrument(skip(websocket_tx, batch))]
async fn send_batch(
    websocket_tx: &mut SplitSink<WebSocket, Message>,
    batch: &[Arc<MessageDTO>],
) -> Result<(), axum::Error> {
    let json = serde_json::to_string(&batch).map_err(|e| {
        error!("Failed to serialize batch: {:?}", e);
        axum::Error::new(e)
    })?;
    websocket_tx
        .send(Message::Text(Utf8Bytes::from(json)))
        .await
        .map_err(|e| {
            error!("Failed to send batch through WebSocket: {:?}", e);
            e
        })
}

#[instrument(skip(websocket_rx, internal_tx, state), fields(user_id))]
pub async fn listen_ws(
    user_id: i32,
    mut websocket_rx: SplitStream<WebSocket>,
    internal_tx: UnboundedSender<InternalSignal>,
    state: Arc<AppState>,
) {
    info!("Listen task started");

    let mut rate_limiter = interval(Duration::from_millis(RATE_LIMITER_MILLIS));
    let timeout_duration = Duration::from_secs(TIMEOUT_DURATION_SECONDS);

    loop {
        match timeout(timeout_duration, StreamExt::next(&mut websocket_rx)).await {
            Ok(Some(msg_result)) => {
                rate_limiter.tick().await;

                let msg = match msg_result {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("WebSocket error: {:?}", e);
                        break;
                    }
                };

                match msg {
                    Message::Text(text) => {
                        if let Ok(event) = serde_json::from_str::<MessageDTO>(&text) {
                            info!("Message received from client");
                            process_message(&state, user_id, event).await;
                        } else {
                            warn!("Failed to deserialize message");
                        }
                    }
                    Message::Close(_) => {
                        info!("Close message received");
                        break;
                    }
                    _ => {}
                }
            }
            Ok(None) => {
                info!("WebSocket stream ended");
                break;
            }
            Err(_) => {
                warn!(
                    timeout_secs = TIMEOUT_DURATION_SECONDS,
                    "Connection timeout"
                );
                break;
            }
        }
    }

    // Cleanup
    info!("Cleaning up connection");
    let _ = internal_tx.send(InternalSignal::Shutdown);
    state.users_online.remove_from_online(&user_id);
    info!("Listen task terminated");
}
