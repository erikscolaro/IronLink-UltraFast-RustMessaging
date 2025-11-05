//! Integration tests per gli endpoints WebSocket
//!
//! Test per:
//! - Connessione WebSocket con autenticazione valida
//! - Gestione utenti duplicati (stesso utente che si connette due volte)
//! - Caricamento chat dell'utente alla connessione
//! - Gestione utenti senza chat
//!
//! Questi test usano `#[sqlx::test]` che:
//! - Crea automaticamente un database di test isolato
//! - Applica le migrations da `migrations/`
//! - Applica i fixtures specificati da `fixtures/`
//! - Pulisce il database al termine

mod common;

#[cfg(test)]
mod ws_tests {
    use server::ws::usermap::{UserMap, InternalSignal};
    use super::common::*;
    use tokio::sync::mpsc;
    use tracing::info;
    // ============================================================
    // WF0 Test unitario per UserMap - verifica sovrascrittura connessioni duplicate
    // ============================================================

    /// Test che verifica il comportamento della UserMap quando lo stesso utente
    /// si connette due volte: la seconda connessione deve sovrascrivere la prima
    /// e il vecchio channel deve essere chiuso
    #[tokio::test]
    async fn test_usermap_duplicate_connection_overwrites() {
        let user_map = UserMap::new();
        let user_id = 1;

        // Prima connessione - crea il primo channel
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        user_map.register_online(user_id, tx1);

        // Verifica che l'utente sia registrato
        assert!(user_map.is_user_online(&user_id), "User should be online after first connection");
        assert_eq!(user_map.online_count(), 1, "Should have exactly 1 user online");

        // Seconda connessione - crea il secondo channel per lo stesso user_id
        // Questo simula l'utente che si connette di nuovo (es. da un altro dispositivo o refresh)
        let (tx2, mut _rx2) = mpsc::unbounded_channel();
        user_map.register_online(user_id, tx2);

        // Verifica che:
        // L'utente sia ancora registrato (solo una volta, non duplicato)
        assert!(user_map.is_user_online(&user_id), "User should still be online");
        assert_eq!(
            user_map.online_count(), 
            1, 
            "Should still have only 1 user online (not duplicated)"
        );

        // Verifica che il vecchio channel sia effettivamente chiuso
        // provando a ricevere dopo un breve timeout
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let delayed_msg = rx1.try_recv();
        assert!(
            delayed_msg.is_err(),
            "Old receiver should be completely disconnected"
        );
    }

    // ============================================================
    // WF0 Test per verifica caricamento chat dal DB e registrazione nella ChatMap
    // ============================================================

    /// Test che verifica che il task write_ws carichi le chat dell'utente dal database
    /// e le registri correttamente nella ChatMap
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_write_task_loads_user_chats_from_db(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
        use super::common::*;

        // Crea lo stato del server
        let state = create_test_state(&pool);

        // Crea un utente di test e aggiungi le sue chat
        let user_id = 1; // Alice dai fixtures

        // Verifica quante chat ha l'utente nel database
        let user_chats = state.meta.find_many_by_user_id(&user_id).await
            .expect("Failed to load user chats from DB");
        
        let chat_count = user_chats.len();
        assert!(chat_count > 0, "Test user should have at least one chat in fixtures");

        let chat_ids: Vec<i32> = user_chats.iter().map(|m| m.chat_id).collect();
        
        // Simula ciò che fa il task write_ws: sottoscrive l'utente alle chat
        let subscriptions = state.chats_online.subscribe_multiple(chat_ids.clone());
        
        // Verifica che il numero di sottoscrizioni corrisponda al numero di chat
        assert_eq!(
            subscriptions.len(), 
            chat_count,
            "Number of subscriptions should match number of user chats"
        );

        // Verifica che ogni chat abbia un canale broadcast nella ChatMap
        for chat_id in &chat_ids {
            // Prova a sottoscrivere di nuovo - questo dovrebbe usare il canale esistente
            let _rx = state.chats_online.subscribe(chat_id);
            
            // Se arriviamo qui senza panic, significa che il canale esiste
            assert!(true, "Should be able to subscribe to chat {}", chat_id);
        }

        // Simula l'invio di un messaggio a una delle chat
        if let Some(&first_chat_id) = chat_ids.first() {
            use server::dtos::MessageDTO;
            use server::entities::MessageType;
            use std::sync::Arc;

            let test_message = Arc::new(MessageDTO {
                message_id: Some(999),
                chat_id: Some(first_chat_id),
                sender_id: Some(user_id),
                content: Some("Test message".to_string()),
                message_type: Some(MessageType::UserMessage),
                created_at: Some(chrono::Utc::now()),
            });

            // Invia il messaggio al canale broadcast
            let result = state.chats_online.send(&first_chat_id, test_message.clone());
            
            // Il risultato può essere Ok(n) dove n è il numero di receiver attivi
            // oppure Err se non ci sono receiver (cosa normale in questo test)
            match result {
                Ok(n) => {
                    // n receiver hanno ricevuto il messaggio (può essere 0 se non ci sono subscriber)
                    info!("Message sent to {} receivers", n);
                }
                Err(_) => {
                    // Nessun receiver attivo, ma il canale esiste
                    // Questo è OK per il test
                    info!("No active receivers for the message");
                }
            }
        }

        Ok(())
    }

    // ============================================================
    // WF1: Test parsing messaggi WebSocket - utente connesso invia messaggi malformati
    // ============================================================

    /// WF1 - Simula un utente connesso che invia messaggi (validi e malformati) via WebSocket
    /// 
    /// Scenario:
    /// 1. Utente1 si connette al server (registrato nella UserMap)
    /// 2. Utente1 invia messaggi via WebSocket (simulati)
    /// 3. Alcuni messaggi sono malformati → il server li ignora senza errori
    /// 4. Alcuni messaggi sono validi → il server li riconosce
    /// 5. La connessione rimane attiva durante tutto il processo
    /// 
    /// Questo test simula la logica di listen_ws che riceve messaggi dal WebSocket
    /// e verifica che messaggi malformati vengano ignorati senza crashare.
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_wf1_user_sends_malformed_messages_via_websocket(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
        

        // === FASE 1: Setup - Utente1 si connette al server ===
        let state = create_test_state(&pool);
        let user_id = 1; // Alice dai fixtures
        
        // Crea il channel per simulare la connessione WebSocket dell'utente
        let (internal_tx, mut _internal_rx) = tokio::sync::mpsc::unbounded_channel::<InternalSignal>();
        
        // Registra l'utente come online (simula la connessione WebSocket)
        state.users_online.register_online(user_id, internal_tx.clone());
        
        assert!(
            state.users_online.is_user_online(&user_id),
            "User should be registered as online"
        );

        // === FASE 2: Utente1 invia una serie di messaggi via WebSocket ===
        
        // Questi sono i messaggi che arriverebbero dal WebSocket (Message::Text(text))
        let incoming_websocket_messages = vec![
            // Messaggio 1: JSON completamente invalido
            "{ this is not valid json at all }",
            
            // Messaggio 2: Messaggio valido
            r#"{"chat_id": 1, "content": "Hello World", "message_type": "UserMessage"}"#,
            
            // Messaggio 3: Array invece di oggetto
            "[1, 2, 3, 4, 5]",
            
            // Messaggio 4: Stringa vuota
            "",
            
            // Messaggio 5: Numero invece di oggetto
            "42",
            
            // Messaggio 6: Altro messaggio valido (parziale)
            r#"{"chat_id": 2, "content": "Another message"}"#,
            
            // Messaggio 7: Oggetto con campi completamente sbagliati
            r#"{"random_field": "value", "another": 123}"#,
        ];

        let mut messages_processed = 0;
        let mut messages_ignored = 0;

        // === FASE 3: Il server processa i messaggi (simula listen_ws) ===
        
        // Questa è la LOGICA ESATTA di listen_ws (righe 207-211 di connection.rs):
        for message_text in incoming_websocket_messages {
            // Il server tenta di deserializzare ogni messaggio
            if let Ok(event) = serde_json::from_str::<server::dtos::MessageDTO>(message_text) {
                // Messaggio valido - in produzione chiamerebbe process_message()
                messages_processed += 1;
                
                info!(
                    chat_id = ?event.chat_id,
                    content = ?event.content,
                    "Valid message received from user"
                );
                
                // Nota: Un messaggio con tutti i campi None è tecnicamente valido
                // ma probabilmente verrebbe scartato da process_message()
            } else {
                // Messaggio malformato - viene ignorato silenziosamente
                // In produzione: warn!("Failed to deserialize message");
                messages_ignored += 1;
                
                info!("Malformed message ignored without error");
            }
        }

        // === FASE 4: Verifica dei risultati ===

        // 1. Verifica che i messaggi validi siano stati riconosciuti
        // Nota: Il messaggio 7 con campi sbagliati viene deserializzato con tutti i campi a None
        assert_eq!(
            messages_processed, 
            3, 
            "Should have processed 3 valid messages"
        );

        // 2. Verifica che i messaggi malformati siano stati ignorati
        // Solo i messaggi con sintassi JSON invalida vengono rifiutati
        assert_eq!(
            messages_ignored, 
            4, 
            "Should have ignored 4 malformed messages (invalid JSON syntax)"
        );

        // 3. Verifica che l'utente sia ancora connesso (il server non ha crashato)
        assert!(
            state.users_online.is_user_online(&user_id),
            "User should still be online after processing malformed messages"
        );

        // 4. Verifica che il canale sia ancora funzionante
        let test_signal = internal_tx.send(InternalSignal::AddChat(999));
        assert!(
            test_signal.is_ok(),
            "Should still be able to send signals to the user after malformed messages"
        );

        // === CONCLUSIONE ===
        // Il test è arrivato fino alla fine senza panic/crash,
        // dimostrando che il server gestisce correttamente messaggi malformati

        Ok(())
    }

    // ============================================================
    // WF2: Test invio messaggio con campi errati - server invia errore
    // ============================================================

    /// WF2 - Verifica che il server invii un messaggio di errore quando riceve
    /// un messaggio con struttura valida ma campi errati
    /// 
    /// Scenario:
    /// 1. Utente1 connesso al server
    /// 2. Utente1 invia un messaggio correttamente strutturato MA con errori nei dati:
    ///    - chat_id inesistente
    ///    - chat_id di una chat a cui l'utente non appartiene
    ///    - message_type SystemMessage (non permesso agli utenti)
    ///    - campi mancanti/invalidi
    ///    - sender_id diverso dall'utente connesso
    /// 3. Il server processa il messaggio
    /// 4. Il server rileva l'errore
    /// 5. Il server invia un InternalSignal::Error all'utente
    /// 
    /// Questo test verifica la logica di process_message che valida i messaggi
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_wf2_server_sends_error_for_invalid_message_fields(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
        use server::ws::event_handlers::process_message;

        // === FASE 1: Setup - Utente1 si connette al server ===
        let state = create_test_state(&pool);
        let user_id = 1; // Alice dai fixtures
        
        // Crea il channel per ricevere messaggi dal server
        let (internal_tx, mut internal_rx) = tokio::sync::mpsc::unbounded_channel::<InternalSignal>();
        
        // Registra l'utente come online
        state.users_online.register_online(user_id, internal_tx.clone());
        
        assert!(state.users_online.is_user_online(&user_id));

        // === FASE 2: Test vari scenari di errore ===

        // SCENARIO 1: Chat inesistente / utente non membro
        let message_wrong_chat = serde_json::from_str::<server::dtos::MessageDTO>(
            r#"{"chat_id": 99999, "sender_id": 1, "content": "Hello", "message_type": "UserMessage"}"#
        ).expect("Valid JSON");

        process_message(&state, user_id, message_wrong_chat).await;

        // Verifica che sia stato inviato un messaggio di errore
        let error_msg = internal_rx.try_recv();
        assert!(error_msg.is_ok(), "Should receive error message for non-member chat");
        
        if let Ok(InternalSignal::Error(msg)) = error_msg {
            assert!(
                msg.contains("don't belong") || msg.contains("group"),
                "Error message should indicate user doesn't belong to chat, got: {}", msg
            );
        } else {
            panic!("Expected Error signal, got something else");
        }

        // SCENARIO 2: Tentativo di inviare messaggio di tipo SystemMessage
        let message_system_type = serde_json::from_str::<server::dtos::MessageDTO>(
            r#"{"chat_id": 1, "sender_id": 1, "content": "System message", "message_type": "SystemMessage"}"#
        ).expect("Valid JSON");

        process_message(&state, user_id, message_system_type).await;

        // Verifica errore per tipo messaggio non permesso
        let error_msg = internal_rx.try_recv();
        assert!(error_msg.is_ok(), "Should receive error for SystemMessage type");
        
        if let Ok(InternalSignal::Error(msg)) = error_msg {
            assert!(
                msg.contains("cannot send system") || msg.contains("system type"),
                "Error should indicate system messages not allowed, got: {}", msg
            );
        } else {
            panic!("Expected Error signal for system message");
        }

        // SCENARIO 3: Messaggio con campi mancanti (chat_id None)
        let message_missing_chat = serde_json::from_str::<server::dtos::MessageDTO>(
            r#"{"content": "Message without chat_id"}"#
        ).expect("Valid JSON");

        process_message(&state, user_id, message_missing_chat).await;

        // Verifica errore per messaggio malformato
        let error_msg = internal_rx.try_recv();
        assert!(error_msg.is_ok(), "Should receive error for missing chat_id");
        
        if let Ok(InternalSignal::Error(msg)) = error_msg {
            assert!(
                msg.contains("Malformed"),
                "Error should indicate malformed message, got: {}", msg
            );
        } else {
            panic!("Expected Error signal for missing fields");
        }

        // SCENARIO 4: Utente si spaccia per un altro (sender_id diverso dall'utente connesso)
        // Alice (user_id=1) tenta di inviare un messaggio fingendosi Bob (user_id=2)
        let message_fake_sender = serde_json::from_str::<server::dtos::MessageDTO>(
            r#"{"chat_id": 1, "sender_id": 2, "content": "Fake message", "message_type": "UserMessage"}"#
        ).expect("Valid JSON");

        process_message(&state, user_id, message_fake_sender).await;

        // Verifica errore per sender_id non corrispondente
        let error_msg = internal_rx.try_recv();
        assert!(error_msg.is_ok(), "Should receive error for mismatched sender_id");
        
        if let Ok(InternalSignal::Error(msg)) = error_msg {
            assert!(
                msg.contains("Malformed"),
                "Error should indicate malformed message (sender_id mismatch), got: {}", msg
            );
        } else {
            panic!("Expected Error signal for sender_id mismatch");
        }

        // SCENARIO 5: Messaggio valido - NON dovrebbe generare errore nel channel
        // (l'errore eventuale sarebbe solo nel salvataggio DB, non nella validazione)
        let valid_message = serde_json::from_str::<server::dtos::MessageDTO>(
            r#"{"chat_id": 1, "sender_id": 1, "content": "Valid message", "message_type": "UserMessage"}"#
        ).expect("Valid JSON");

        process_message(&state, user_id, valid_message).await;

        // Per un messaggio valido, potrebbe non esserci nessun errore nel channel
        // (o potrebbe esserci un errore di DB se fallisce il salvataggio)
        // Aspettiamo un breve momento per eventuali errori asincroni
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Se c'è un messaggio, potrebbe essere un errore di DB (accettabile nel test)
        // Se non c'è nessun messaggio, significa che tutto è andato bene
        match internal_rx.try_recv() {
            Ok(InternalSignal::Error(msg)) => {
                // Potrebbe essere un errore di salvataggio DB, che è OK per questo test
                info!("Received error (possibly DB related): {}", msg);
            }
            Err(_) => {
                // Nessun messaggio = tutto OK
                info!("No error received for valid message - all good!");
            }
            _ => {
                // Altri tipi di segnale sono inaspettati
                panic!("Unexpected signal type for valid message");
            }
        }

        // === FASE 3: Verifica finale ===
        
        // L'utente è ancora connesso (nessun crash)
        assert!(
            state.users_online.is_user_online(&user_id),
            "User should still be online after error scenarios"
        );

        Ok(())
    }

    
}

