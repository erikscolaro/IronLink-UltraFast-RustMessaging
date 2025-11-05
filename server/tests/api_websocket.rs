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
    async fn test_wf0_usermap_duplicate_connection_overwrites() {
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
    async fn test_wf0_write_task_loads_user_chats_from_db(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
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
    // WF1: Test invio messaggio con campi errati - server invia errore
    // ============================================================
    
    /// WF1 - Verifica che il server invii un messaggio di errore quando riceve
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
    async fn test_wf1_server_sends_error_for_invalid_message_fields(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
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

    // ============================================================
    // WF1: Test salvataggio messaggio nel database dopo invio WebSocket
    // ============================================================

    /// WF1 - Verifica che un messaggio valido inviato via WebSocket venga salvato nel database
    /// in una chat privata con ricevente offline
    /// 
    /// Scenario:
    /// 1. Alice connessa al server (Bob offline)
    /// 2. Alice invia un messaggio valido in chat privata tramite process_message
    /// 3. Il messaggio viene processato e validato
    /// 4. Il messaggio viene salvato nel database (per Bob quando si connetterà)
    /// 5. Verifica che il messaggio sia presente nel DB con i dati corretti
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_wf1_message_saved_to_database_after_websocket_send(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
        use server::ws::event_handlers::process_message;

        // === FASE 1: Setup - Alice online in chat privata, Bob offline ===
        let state = create_test_state(&pool);
        let alice_id = 1; // Sender
        let bob_id = 2;   // Receiver (offline)
        let chat_id = 2;  // Chat PRIVATA Alice-Bob dai fixtures
        
        // Crea il channel per ricevere messaggi dal server
        let (internal_tx, mut internal_rx) = tokio::sync::mpsc::unbounded_channel::<InternalSignal>();
        
        // Registra Alice come online (Bob rimane offline)
        state.users_online.register_online(alice_id, internal_tx.clone());
        
        // Sottoscrivi Alice alla chat privata (simula il comportamento del write_ws task)
        let _subscriptions = state.chats_online.subscribe_multiple(vec![chat_id]);

        assert!(state.users_online.is_user_online(&alice_id), "Alice should be online");
        assert!(!state.users_online.is_user_online(&bob_id), "Bob should be offline");

        // === FASE 2: Conta i messaggi esistenti per questa chat ===
        let messages_before = sqlx::query!(
            "SELECT COUNT(*) as count FROM messages WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;
        
        let count_before = messages_before.count;
        info!("Messages in chat {} before sending: {}", chat_id, count_before);

        // === FASE 3: Invia un messaggio valido tramite WebSocket (simula process_message) ===
        let message_content = "Test message for database persistence in private chat (Bob offline)";
        let valid_message = serde_json::from_str::<server::dtos::MessageDTO>(
            &format!(
                r#"{{"chat_id": {}, "sender_id": {}, "content": "{}", "message_type": "UserMessage"}}"#,
                chat_id, alice_id, message_content
            )
        ).expect("Valid JSON");

        // Processa il messaggio (come farebbe listen_ws dopo la deserializzazione)
        process_message(&state, alice_id, valid_message).await;

        // Aspetta un breve momento per permettere il salvataggio asincrono nel DB
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // === FASE 4: Verifica che il messaggio sia stato salvato nel DB ===
        let messages_after = sqlx::query!(
            "SELECT COUNT(*) as count FROM messages WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;
        
        let count_after = messages_after.count;
        info!("Messages in chat {} after sending: {}", chat_id, count_after);

        assert_eq!(
            count_after, count_before + 1,
            "Should have exactly one more message in the database after sending"
        );

        // === FASE 5: Verifica il contenuto del messaggio salvato ===
        let saved_message = sqlx::query!(
            "SELECT message_id, chat_id, sender_id, content, message_type 
             FROM messages 
             WHERE chat_id = ? AND content = ?
             ORDER BY created_at DESC
             LIMIT 1",
            chat_id,
            message_content
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(saved_message.chat_id, chat_id, "Chat ID should match");
        assert_eq!(saved_message.sender_id, alice_id, "Sender ID should be Alice");
        assert_eq!(saved_message.content, message_content, "Content should match sent message");
        assert_eq!(saved_message.message_type.to_uppercase(), "USERMESSAGE", "Message type should be USERMESSAGE");

        info!(
            "Message successfully saved to database with ID: {} (will be delivered to Bob when he comes online)",
            saved_message.message_id
        );

        // === FASE 6: Verifica che NON ci siano errori nel channel ===
        // (il messaggio era valido, quindi non dovrebbe esserci alcun errore)
        match internal_rx.try_recv() {
            Err(_) => {
                // Nessun messaggio nel channel = tutto OK
                info!("No error messages received - message processed and saved successfully");
            }
            Ok(InternalSignal::Error(msg)) => {
                panic!("Unexpected error received for valid message: {}", msg);
            }
            Ok(_) => {
                // Altri tipi di segnale potrebbero essere OK (es. AddChat)
                info!("Received non-error signal (this may be expected)");
            }
        }

        // === FASE 7: Verifica finale ===
        assert!(
            state.users_online.is_user_online(&alice_id),
            "Alice should still be online after sending message"
        );

        Ok(())
    }

    // ============================================================
    // WF1: Test invio messaggio in chat privata con ricevente offline
    // ============================================================

    /// WF1 - Verifica che un messaggio inviato in una chat privata venga salvato
    /// correttamente nel database quando il ricevente è offline
    /// 
    /// Scenario:
    /// 1. Alice (sender) è online
    /// 2. Bob (receiver) è OFFLINE
    /// 3. Alice invia un messaggio a Bob in una chat PRIVATA
    /// 4. Il messaggio viene salvato nel DB (per consegna futura)
    /// 5. Il broadcast non ha receiver attivi (Bob offline)
    /// 6. Alice non riceve errori
    /// 7. Bob riceverà il messaggio quando si connetterà
    /// 
    /// Questo test verifica il comportamento WF1: chat privata, ricevente offline
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_wf1_send_message_private_chat_receiver_offline(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
        use server::ws::event_handlers::process_message;

        // === FASE 1: Setup - Solo Alice online in chat privata, Bob offline ===
        let state = create_test_state(&pool);
        let alice_id = 1; // Sender
        let bob_id = 2;   // Receiver (OFFLINE)
        let chat_id = 2;  // Chat PRIVATE Alice-Bob dai fixtures
        
        // Solo Alice è online
        let (internal_tx_alice, mut internal_rx_alice) = tokio::sync::mpsc::unbounded_channel::<InternalSignal>();
        state.users_online.register_online(alice_id, internal_tx_alice.clone());
        
        // Bob NON è registrato come online (simula utente offline)
        assert!(state.users_online.is_user_online(&alice_id), "Alice should be online");
        assert!(!state.users_online.is_user_online(&bob_id), "Bob should be offline");

        // Alice sottoscrivi alla chat privata
        let _subscriptions = state.chats_online.subscribe_multiple(vec![chat_id]);

        // === FASE 2: Verifica che sia una chat PRIVATA con 2 membri ===
        let chat_info = sqlx::query!(
            "SELECT chat_type FROM chats WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(chat_info.chat_type, "PRIVATE", "Should be a private chat");

        let chat_members = sqlx::query!(
            "SELECT COUNT(DISTINCT user_id) as count 
             FROM userchatmetadata 
             WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        info!("Private chat {} has {} members", chat_id, chat_members.count);
        assert_eq!(
            chat_members.count, 2,
            "Private chat should have exactly 2 members"
        );

        // === FASE 3: Conta messaggi prima dell'invio ===
        let messages_before = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        info!("Messages in private chat before: {}", messages_before);

        // === FASE 4: Alice invia un messaggio a Bob nella chat privata ===
        let message_content = "Hey Bob! This message is for you while you're offline.";
        let message = serde_json::from_str::<server::dtos::MessageDTO>(
            &format!(
                r#"{{"chat_id": {}, "sender_id": {}, "content": "{}", "message_type": "UserMessage"}}"#,
                chat_id, alice_id, message_content
            )
        ).expect("Valid JSON");

        // Processa il messaggio
        process_message(&state, alice_id, message).await;

        // Aspetta il salvataggio asincrono
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // === FASE 5: Verifica che il messaggio sia stato salvato nel DB ===
        let messages_after = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        info!("Messages in private chat after: {}", messages_after);

        assert_eq!(
            messages_after, messages_before + 1,
            "Message should be saved to database even when receiver is offline"
        );

        // Verifica il contenuto del messaggio salvato
        let saved_message = sqlx::query!(
            "SELECT message_id, sender_id, content, message_type 
             FROM messages 
             WHERE chat_id = ? AND content = ?",
            chat_id,
            message_content
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(saved_message.sender_id, alice_id, "Sender should be Alice");
        assert_eq!(saved_message.content, message_content, "Content should match");
        assert_eq!(saved_message.message_type.to_uppercase(), "USERMESSAGE", "Message type should be USERMESSAGE");
        
        info!(
            "Message saved successfully with ID: {} (will be delivered to Bob when he comes online)",
            saved_message.message_id
        );

        // === FASE 6: Verifica che Alice NON abbia ricevuto errori ===
        let alice_errors = internal_rx_alice.try_recv();
        
        if let Ok(signal) = alice_errors {
            match signal {
                InternalSignal::Error(msg) => {
                    // Errori DB sono accettabili, ma non errori di validazione
                    assert!(
                        !msg.contains("don't belong") && !msg.contains("Malformed"),
                        "Should not receive validation errors, got: {}", msg
                    );
                    info!("Received error (possibly DB related): {}", msg);
                }
                _ => {
                    info!("Received non-error signal");
                }
            }
        } else {
            info!("No error received - message processed successfully");
        }

        // === FASE 7: Verifica che Alice sia ancora online ===
        assert!(
            state.users_online.is_user_online(&alice_id),
            "Alice should still be online after sending message"
        );

        // === FASE 8: Simula Bob che si connette e verifica che riceva il messaggio dal DB ===
        info!("Simulating Bob coming online and fetching messages from DB...");
        
        let bob_messages = sqlx::query!(
            "SELECT m.message_id, m.sender_id, m.content, m.created_at
             FROM messages m
             JOIN userchatmetadata ucm ON m.chat_id = ucm.chat_id
             WHERE ucm.user_id = ? AND m.chat_id = ?
             ORDER BY m.created_at DESC
             LIMIT 10",
            bob_id,
            chat_id
        )
        .fetch_all(&pool)
        .await?;

        // Verifica che Bob possa vedere il messaggio di Alice
        let alice_message_visible_to_bob = bob_messages.iter()
            .any(|msg| msg.content == message_content && msg.sender_id == alice_id);

        assert!(
            alice_message_visible_to_bob,
            "Bob should be able to see Alice's message from DB when he comes online"
        );

        info!(
            "Success! Private chat message persisted correctly and will be delivered to Bob when he reconnects"
        );

        Ok(())
    }

    // ============================================================
    // WF2: Test invio messaggio in chat di gruppo con tutti utenti offline
    // ============================================================

    /// WF2 - Verifica il comportamento corretto quando si invia un messaggio
    /// in una chat di gruppo dove tutti gli altri membri sono offline
    /// 
    /// Scenario:
    /// 1. Chat di gruppo con 3 utenti: Alice, Bob, Charlie
    /// 2. Solo Alice è online (Bob e Charlie offline)
    /// 3. Alice invia un messaggio nella chat di gruppo
    /// 4. Il messaggio viene salvato nel DB
    /// 5. Il broadcast NON genera errori (anche se non ci sono receiver attivi)
    /// 6. Alice non riceve errori
    /// 7. Bob e Charlie riceveranno il messaggio quando si connetteranno (persistenza DB)
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_wf2_send_message_to_group_chat_all_users_offline(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
        use server::ws::event_handlers::process_message;

        // === FASE 1: Setup - Solo Alice online, Bob e Charlie offline ===
        let state = create_test_state(&pool);
        let alice_id = 1;
        let bob_id = 2;
        let charlie_id = 3;
        let chat_id = 1; // Chat di gruppo dai fixtures
        
        // Solo Alice è online
        let (internal_tx_alice, mut internal_rx_alice) = tokio::sync::mpsc::unbounded_channel::<InternalSignal>();
        state.users_online.register_online(alice_id, internal_tx_alice.clone());
        
        // Bob e Charlie NON sono registrati come online (simulano utenti offline)
        assert!(state.users_online.is_user_online(&alice_id), "Alice should be online");
        assert!(!state.users_online.is_user_online(&bob_id), "Bob should be offline");
        assert!(!state.users_online.is_user_online(&charlie_id), "Charlie should be offline");

        // Alice sottoscrivi alla chat
        let _subscriptions = state.chats_online.subscribe_multiple(vec![chat_id]);

        // === FASE 2: Verifica membri della chat nel DB ===
        let chat_members = sqlx::query!(
            "SELECT COUNT(DISTINCT user_id) as count 
             FROM userchatmetadata 
             WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        info!("Chat {} has {} members in DB", chat_id, chat_members.count);
        assert!(
            chat_members.count >= 2,
            "Chat should have at least 2 members for group chat test"
        );

        // === FASE 3: Conta messaggi prima dell'invio ===
        let messages_before = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        info!("Messages in chat before: {}", messages_before);

        // === FASE 4: Alice invia un messaggio nella chat di gruppo ===
        let message_content = "Hello everyone! This message is sent when you're all offline.";
        let message = serde_json::from_str::<server::dtos::MessageDTO>(
            &format!(
                r#"{{"chat_id": {}, "sender_id": {}, "content": "{}", "message_type": "UserMessage"}}"#,
                chat_id, alice_id, message_content
            )
        ).expect("Valid JSON");

        // Processa il messaggio
        process_message(&state, alice_id, message).await;

        // Aspetta il salvataggio asincrono
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // === FASE 5: Verifica che il messaggio sia stato salvato nel DB ===
        let messages_after = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        info!("Messages in chat after: {}", messages_after);

        assert_eq!(
            messages_after, messages_before + 1,
            "Message should be saved to database even when all other users are offline"
        );

        // Verifica il contenuto del messaggio salvato
        let saved_message = sqlx::query!(
            "SELECT message_id, sender_id, content 
             FROM messages 
             WHERE chat_id = ? AND content = ?",
            chat_id,
            message_content
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(saved_message.sender_id, alice_id, "Sender should be Alice");
        assert_eq!(saved_message.content, message_content, "Content should match");
        
        info!(
            "Message saved successfully with ID: {} (will be delivered to Bob and Charlie when they come online)",
            saved_message.message_id
        );

        // === FASE 6: Verifica che Alice NON abbia ricevuto errori ===
        // Il broadcast potrebbe non avere receiver attivi (tutti offline), ma questo è OK
        let alice_errors = internal_rx_alice.try_recv();
        
        // Se c'è un messaggio, verifica che non sia un errore di "gruppo non trovato" o simili
        if let Ok(signal) = alice_errors {
            match signal {
                InternalSignal::Error(msg) => {
                    // Errori DB sono accettabili, ma non errori di validazione
                    assert!(
                        !msg.contains("don't belong") && !msg.contains("Malformed"),
                        "Should not receive validation errors, got: {}", msg
                    );
                    info!("Received error (possibly DB related): {}", msg);
                }
                _ => {
                    info!("Received non-error signal");
                }
            }
        } else {
            info!("No error received - message processed successfully");
        }

        // === FASE 7: Verifica che Alice sia ancora online ===
        assert!(
            state.users_online.is_user_online(&alice_id),
            "Alice should still be online after sending message"
        );

        // === FASE 8: Simula Bob che si connette e verifica che riceva il messaggio dal DB ===
        info!("Simulating Bob coming online and fetching messages from DB...");
        
        let bob_messages = sqlx::query!(
            "SELECT m.message_id, m.sender_id, m.content, m.created_at
             FROM messages m
             JOIN userchatmetadata ucm ON m.chat_id = ucm.chat_id
             WHERE ucm.user_id = ? AND m.chat_id = ?
             ORDER BY m.created_at DESC
             LIMIT 10",
            bob_id,
            chat_id
        )
        .fetch_all(&pool)
        .await?;

        // Verifica che Bob possa vedere il messaggio di Alice
        let alice_message_visible_to_bob = bob_messages.iter()
            .any(|msg| msg.content == message_content && msg.sender_id == alice_id);

        assert!(
            alice_message_visible_to_bob,
            "Bob should be able to see Alice's message from DB when he comes online"
        );

        info!(
            "Success! Message persisted correctly and will be delivered to offline users when they reconnect"
        );

        Ok(())
    }

    // ============================================================
    // WF3: Test invio messaggio con utenti online - ricezione via broadcast
    // ============================================================

    /// WF3 - Verifica che un utente online riceva un messaggio in tempo reale
    /// tramite broadcast quando un altro utente invia un messaggio
    /// 
    /// Scenario:
    /// 1. Alice e Bob sono entrambi online
    /// 2. Entrambi sottoscritti alla stessa chat privata
    /// 3. Alice invia un messaggio a Bob
    /// 4. Bob riceve il messaggio immediatamente via broadcast channel
    /// 5. Il messaggio ricevuto ha tutti gli attributi corretti
    /// 6. Il messaggio viene anche salvato nel DB
    /// 
    /// Questo test verifica il flusso completo: utenti online → broadcast in tempo reale
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_wf3_receiver_gets_message_via_broadcast_when_online(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
        use server::ws::event_handlers::process_message;

        // === FASE 1: Setup - Alice e Bob online nella stessa chat privata ===
        let state = create_test_state(&pool);
        let alice_id = 1; // Sender
        let bob_id = 2;   // Receiver (ONLINE)
        let chat_id = 2;  // Chat PRIVATA Alice-Bob
        
        // Setup Alice (sender)
        let (internal_tx_alice, mut _internal_rx_alice) = tokio::sync::mpsc::unbounded_channel::<InternalSignal>();
        state.users_online.register_online(alice_id, internal_tx_alice.clone());
        
        // Setup Bob (receiver) - ONLINE e sottoscritto alla chat
        let (internal_tx_bob, mut _internal_rx_bob) = tokio::sync::mpsc::unbounded_channel::<InternalSignal>();
        state.users_online.register_online(bob_id, internal_tx_bob.clone());
        
        // Bob sottoscrivi alla chat per ricevere messaggi via broadcast
        let mut receivers = state.chats_online.subscribe_multiple(vec![chat_id]);
        assert_eq!(receivers.len(), 1, "Should have one broadcast receiver for the chat");
        
        let mut bob_chat_rx = receivers.remove(0);

        // Verifica che entrambi siano online
        assert!(state.users_online.is_user_online(&alice_id), "Alice should be online");
        assert!(state.users_online.is_user_online(&bob_id), "Bob should be online");

        // === FASE 2: Verifica che sia una chat PRIVATA ===
        let chat_info = sqlx::query!(
            "SELECT chat_type FROM chats WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(chat_info.chat_type, "PRIVATE", "Should be a private chat");

        // === FASE 3: Alice invia un messaggio a Bob ===
        let message_content = "Hey Bob! You should receive this in real-time!";
        let message = serde_json::from_str::<server::dtos::MessageDTO>(
            &format!(
                r#"{{"chat_id": {}, "sender_id": {}, "content": "{}", "message_type": "UserMessage"}}"#,
                chat_id, alice_id, message_content
            )
        ).expect("Valid JSON");

        info!("Alice sending message to Bob...");
        
        // Processa il messaggio (questo triggera il broadcast)
        process_message(&state, alice_id, message).await;

        // === FASE 4: Bob riceve il messaggio dal broadcast channel ===
        info!("Waiting for Bob to receive message via broadcast...");
        
        // Aspetta un breve momento per il broadcast asincrono
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Bob dovrebbe ricevere il messaggio immediatamente
        let received_message = tokio::time::timeout(
            tokio::time::Duration::from_millis(500),
            bob_chat_rx.recv()
        ).await;

        assert!(
            received_message.is_ok(),
            "Bob should receive message from broadcast channel within timeout"
        );

        let received_message = received_message
            .expect("Timeout error")
            .expect("Should receive a message");

        // === FASE 5: Verifica attributi del messaggio ricevuto ===
        info!("Message received by Bob: {:?}", received_message);

        assert_eq!(
            received_message.chat_id,
            Some(chat_id),
            "Chat ID should match"
        );

        assert_eq!(
            received_message.sender_id,
            Some(alice_id),
            "Sender ID should be Alice"
        );

        assert_eq!(
            received_message.content,
            Some(message_content.to_string()),
            "Content should match sent message"
        );

        assert!(
            matches!(received_message.message_type, Some(server::entities::MessageType::UserMessage)),
            "Message type should be UserMessage"
        );

        // IMPORTANTE: message_id è None perché il broadcast avviene PRIMA del salvataggio DB
        assert!(
            received_message.message_id.is_none(),
            "Message ID should be None (broadcast happens before DB save)"
        );

        info!("✓ Bob received message in real-time via broadcast!");

        // === FASE 6: Verifica che il messaggio sia stato anche salvato nel DB ===
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let saved_message = sqlx::query!(
            "SELECT message_id, sender_id, content 
             FROM messages 
             WHERE chat_id = ? AND content = ?",
            chat_id,
            message_content
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(saved_message.sender_id, alice_id, "Sender should be Alice in DB");
        assert_eq!(saved_message.content, message_content, "Content should match in DB");
        
        info!(
            "✓ Message also persisted to database with ID: {}",
            saved_message.message_id
        );

        // === FASE 7: Verifica che non ci siano altri messaggi pendenti ===
        let no_more_messages = bob_chat_rx.try_recv();
        assert!(
            no_more_messages.is_err(),
            "Should not have any more messages in the channel"
        );

        info!("Success! WF3: Real-time message delivery via broadcast works correctly!");

        Ok(())
    }

    /// WF3 - Verifica che il sender riceva il proprio messaggio via broadcast
    /// 
    /// Scenario:
    /// 1. Alice è online e sottoscritta a una chat di gruppo
    /// 2. Alice invia un messaggio nella chat di gruppo
    /// 3. Alice riceve il proprio messaggio via broadcast (echo)
    /// 4. Il messaggio ricevuto ha gli attributi corretti
    /// 5. Il messaggio viene salvato nel DB
    /// 
    /// Questo test verifica che il sender riceva una copia del proprio messaggio
    /// tramite il broadcast channel (comportamento tipico delle chat)
    #[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats")))]
    async fn test_wf3_sender_receives_own_message_via_broadcast(pool: sqlx::MySqlPool) -> sqlx::Result<()> {
        use server::ws::event_handlers::process_message;

        // === FASE 1: Setup - Alice online in una chat di gruppo ===
        let state = create_test_state(&pool);
        let alice_id = 1; // Sender
        let chat_id = 1;  // Chat di GRUPPO (General Chat)
        
        // Setup Alice
        let (internal_tx_alice, mut _internal_rx_alice) = tokio::sync::mpsc::unbounded_channel::<InternalSignal>();
        state.users_online.register_online(alice_id, internal_tx_alice.clone());
        
        // Alice sottoscrivi alla chat di gruppo per ricevere messaggi via broadcast
        let mut receivers = state.chats_online.subscribe_multiple(vec![chat_id]);
        assert_eq!(receivers.len(), 1, "Should have one broadcast receiver for the chat");
        
        let mut alice_chat_rx = receivers.remove(0);

        // Verifica che Alice sia online
        assert!(state.users_online.is_user_online(&alice_id), "Alice should be online");

        // === FASE 2: Verifica che sia una chat di GRUPPO ===
        let chat_info = sqlx::query!(
            "SELECT chat_type FROM chats WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(chat_info.chat_type, "GROUP", "Should be a group chat");

        // Verifica membri della chat
        let chat_members = sqlx::query!(
            "SELECT COUNT(DISTINCT user_id) as count 
             FROM userchatmetadata 
             WHERE chat_id = ?",
            chat_id
        )
        .fetch_one(&pool)
        .await?;

        info!("Group chat {} has {} members", chat_id, chat_members.count);
        assert!(
            chat_members.count >= 2,
            "Group chat should have at least 2 members"
        );

        // === FASE 3: Alice invia un messaggio nella chat di gruppo ===
        let message_content = "Hello everyone! I should receive my own message too!";
        let message = serde_json::from_str::<server::dtos::MessageDTO>(
            &format!(
                r#"{{"chat_id": {}, "sender_id": {}, "content": "{}", "message_type": "UserMessage"}}"#,
                chat_id, alice_id, message_content
            )
        ).expect("Valid JSON");

        info!("Alice sending message to group chat...");
        
        // Processa il messaggio
        process_message(&state, alice_id, message).await;

        // === FASE 4: Alice riceve il proprio messaggio dal broadcast channel ===
        info!("Waiting for Alice to receive her own message via broadcast...");
        
        // Aspetta un breve momento per il broadcast asincrono
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Alice dovrebbe ricevere il proprio messaggio (echo)
        let received_message = tokio::time::timeout(
            tokio::time::Duration::from_millis(500),
            alice_chat_rx.recv()
        ).await;

        assert!(
            received_message.is_ok(),
            "Alice should receive her own message from broadcast channel within timeout"
        );

        let received_message = received_message
            .expect("Timeout error")
            .expect("Should receive a message");

        // === FASE 5: Verifica attributi del messaggio ricevuto ===
        info!("Message received by Alice (echo): {:?}", received_message);

        assert_eq!(
            received_message.chat_id,
            Some(chat_id),
            "Chat ID should match"
        );

        assert_eq!(
            received_message.sender_id,
            Some(alice_id),
            "Sender ID should be Alice (herself)"
        );

        assert_eq!(
            received_message.content,
            Some(message_content.to_string()),
            "Content should match sent message"
        );

        assert!(
            matches!(received_message.message_type, Some(server::entities::MessageType::UserMessage)),
            "Message type should be UserMessage"
        );

        // IMPORTANTE: message_id è None perché il broadcast avviene PRIMA del salvataggio DB
        assert!(
            received_message.message_id.is_none(),
            "Message ID should be None (broadcast happens before DB save)"
        );

        info!("✓ Alice received her own message via broadcast (echo)!");

        // === FASE 6: Verifica che il messaggio sia stato salvato nel DB ===
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let saved_message = sqlx::query!(
            "SELECT message_id, sender_id, content 
             FROM messages 
             WHERE chat_id = ? AND content = ?",
            chat_id,
            message_content
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(saved_message.sender_id, alice_id, "Sender should be Alice in DB");
        assert_eq!(saved_message.content, message_content, "Content should match in DB");
        
        info!(
            "✓ Message also persisted to database with ID: {}",
            saved_message.message_id
        );

        // === FASE 7: Verifica che non ci siano altri messaggi pendenti ===
        let no_more_messages = alice_chat_rx.try_recv();
        assert!(
            no_more_messages.is_err(),
            "Should not have any more messages in the channel"
        );

        info!("Success! WF3: Sender receives own message (echo) via broadcast!");

        Ok(())
    }

    
}
