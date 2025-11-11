// WebSocket persistente per Ruggine
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

/// Stato del WebSocket condiviso
struct WebSocketState {
    sender: Arc<Mutex<Option<mpsc::UnboundedSender<String>>>>,
}

/// Messaggio da inviare al WebSocket
#[derive(Serialize, Deserialize, Clone, Debug)]
struct WebSocketMessage {
    chat_id: Option<i32>,
    sender_id: Option<i32>,
    content: Option<String>,
    message_type: Option<String>,
    created_at: Option<String>,
}

/// Comando per connettersi al WebSocket
#[tauri::command]
async fn connect_websocket(
    ws_url: String,
    token: String,
    app_handle: AppHandle,
    state: State<'_, WebSocketState>,
) -> Result<String, String> {
    println!("Tentativo di connessione WebSocket a: {}", ws_url);

    // Crea la richiesta HTTP con l'header Authorization
    use tokio_tungstenite::tungstenite::http::Request;
    
    // Estrai l'host dall'URL WebSocket
    // Formato: ws://host:port/path -> host:port
    let host = ws_url
        .trim_start_matches("ws://")
        .trim_start_matches("wss://")
        .split('/')
        .next()
        .unwrap_or("localhost:3000");
    
    println!("Host estratto: {}", host);
    
    let request = Request::builder()
        .uri(&ws_url)
        .header("Host", host)
        .header("Authorization", format!("Bearer {}", token))
        .header("Sec-WebSocket-Version", "13")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Key", tokio_tungstenite::tungstenite::handshake::client::generate_key())
        .body(())
        .map_err(|e| format!("Errore creazione richiesta: {}", e))?;
    
    // Crea un canale per inviare messaggi al WebSocket
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    
    // Salva il sender nello stato
    {
        let mut sender = state.sender.lock().unwrap();
        *sender = Some(tx);
    }

    // Spawn task per gestire la connessione WebSocket
    tokio::spawn(async move {
        match connect_async(request).await {
            Ok((ws_stream, _)) => {
                println!("WebSocket connesso con successo!");
                
                // Emetti evento di connessione
                let _ = app_handle.emit("ws-connected", ());
                
                let (mut write, mut read) = ws_stream.split();

                // Task per ricevere messaggi dal WebSocket
                let app_handle_read = app_handle.clone();
                let read_task = tokio::spawn(async move {
                    while let Some(message) = read.next().await {
                        match message {
                            Ok(Message::Text(text)) => {
                                println!("Messaggio ricevuto: {}", text);
                                // Emetti evento con il messaggio ricevuto
                                let _ = app_handle_read.emit("ws-message", text);
                            }
                            Ok(Message::Pong(_)) => {
                                println!("Pong ricevuto dal server");
                            }
                            Ok(Message::Close(_)) => {
                                println!("WebSocket chiuso dal server");
                                let _ = app_handle_read.emit("ws-disconnected", ());
                                break;
                            }
                            Err(e) => {
                                eprintln!("Errore ricezione messaggio: {}", e);
                                let _ = app_handle_read.emit("ws-error", format!("{}", e));
                                break;
                            }
                            _ => {}
                        }
                    }
                });

                // Task per inviare messaggi al WebSocket con ping periodici
                let app_handle_write = app_handle.clone();
                let write_task = tokio::spawn(async move {
                    use tokio::time::{interval, Duration};
                    let mut ping_interval = interval(Duration::from_secs(30));
                    ping_interval.tick().await; // Consuma il primo tick
                    
                    loop {
                        tokio::select! {
                            // Messaggio da inviare
                            Some(msg) = rx.recv() => {
                                if let Err(e) = write.send(Message::Text(msg)).await {
                                    eprintln!("Errore invio messaggio: {}", e);
                                    let _ = app_handle_write.emit("ws-error", format!("{}", e));
                                    break;
                                }
                            }
                            // Ping periodico ogni 30 secondi
                            _ = ping_interval.tick() => {
                                if let Err(e) = write.send(Message::Ping(vec![])).await {
                                    eprintln!("Errore invio ping: {}", e);
                                    let _ = app_handle_write.emit("ws-error", format!("{}", e));
                                    break;
                                }
                                println!("Ping inviato al server");
                            }
                        }
                    }
                });

                // Attendi che entrambi i task finiscano
                let _ = tokio::join!(read_task, write_task);
                
                println!("WebSocket disconnesso");
                let _ = app_handle.emit("ws-disconnected", ());
            }
            Err(e) => {
                eprintln!("Errore connessione WebSocket: {}", e);
                let _ = app_handle.emit("ws-error", format!("{}", e));
            }
        }
    });

    Ok("WebSocket connection started".to_string())
}

/// Comando per inviare un messaggio tramite WebSocket
#[tauri::command]
async fn send_websocket_message(
    message: String,
    state: State<'_, WebSocketState>,
) -> Result<(), String> {
    println!("Invio messaggio: {}", message);
    
    let sender = state.sender.lock().unwrap();
    
    if let Some(tx) = sender.as_ref() {
        tx.send(message)
            .map_err(|e| format!("Errore invio messaggio: {}", e))?;
        Ok(())
    } else {
        Err("WebSocket non connesso".to_string())
    }
}

/// Comando per disconnettere il WebSocket
#[tauri::command]
async fn disconnect_websocket(
    state: State<'_, WebSocketState>,
) -> Result<(), String> {
    println!("Disconnessione WebSocket");
    
    let mut sender = state.sender.lock().unwrap();
    *sender = None;
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(WebSocketState {
            sender: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            connect_websocket,
            send_websocket_message,
            disconnect_websocket
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
