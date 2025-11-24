# Manuale Utente — Ruggine


Indice
1. Introduzione
2. Requisiti di sistema
3. Installazione e Configurazione
4. Panoramica dell'interfaccia utente
5. Come utilizzare la Chat
6. Sicurezza e Privacy
7. FAQ e risoluzione problemi rapida
8. Appendice: comandi rapidi e contatti

---

## 1. Introduzione

### Cos’è l’applicazione
Ruggine è un’applicazione di messaggistica in tempo reale che combina API REST e WebSocket per fornire chat private e di gruppo, inviti, gestione ruoli e notifiche. Il backend è scritto in Rust (Axum + sqlx) e il client è React/TypeScript (Vite). In ambienti desktop è possibile usare il pacchetto Tauri se disponibile.

### A chi è destinata
L’app è pensata per utenti finali che desiderano comunicare in tempo reale in contesti di team o privati. È adatta ad utenti non tecnici e a sviluppatori che vogliano estenderla o integrarla.

### Funzionalità principali
- Messaggistica in tempo reale via WebSocket
- Chat private e di gruppo
- Inviti e gestione membership (OWNER, ADMIN, MEMBER)
- Storico messaggi persistito su MySQL
- Notifiche in‑app e browser

---

## 2. Requisiti di sistema

### Dispositivi supportati
- Desktop e laptop (Windows, macOS, Linux)
- Browser moderni: Chrome, Edge, Firefox, Safari (ultime versioni consigliate)
- App desktop (se distribuita tramite Tauri): Windows x86_64, macOS, Linux

### Compatibilità con sistemi operativi
- Client web: qualsiasi sistema con browser moderno
- Server: sistema con toolchain Rust per build (developer). Per l’esercizio quotidiano basta un server Linux/Windows con MySQL e supporto per servizi HTTP/HTTPS.

### Requisiti di rete/connessione
- Connessione internet stabile con porte HTTP/HTTPS aperte
- Supporto WebSocket (Upgrade HTTP → WS) sul reverse proxy o server
- Banda e latenza: connessioni più veloci e latenza bassa migliorano l’esperienza in tempo reale

---

## 3. Installazione e Configurazione

Questa sezione descrive i passaggi per un utente finale; gli amministratori di sistema troveranno istruzioni di deploy nel file `docs/RUGGINE_DOC.md`.

### Come creare un account
1. Apri l’app nel browser (URL fornito dall’amministratore) o avvia il client desktop.
2. Clic su `Registrati`.
3. Inserisci un nome utente e una password conforme alla policy (min. 8 caratteri, e deve contenere almeno: una lettera minuscola e una maiuscola, un numero e un carattere speciale).
4. Clic su `Crea account`.
5. In caso di successo sarai reindirizzato alla schermata di login.

### Procedura di login
1. Apri la pagina di login.
2. Inserisci username e password.
3. Clic su `Accedi` o premi Enter.
4. Se le credenziali sono corrette, l’app riceverà un token di sessione (JWT) e ti dirigerà alla Home.

### Configurazione iniziale
- Imposta l’immagine del profilo (se disponibile)
- Controlla le impostazioni di notifica (browser o app)
- Aggiungi i contatti o aspetta inviti

Nota: la gestione delle impostazioni è disponibile nella sezione **Impostazioni** (vedi capitolo 4).

---

## 4. Panoramica dell’interfaccia utente

La UI è organizzata in modo da essere familiare a chi usa applicazioni di chat moderne.

### Elementi principali
- **Sidebar (sinistra)**: elenco chat e contatti, pulsante `Nuova chat`/`Crea gruppo`, campo ricerca.
- **Area centrale**: conversazione attiva, elenco messaggi.
- **Header / Dettagli (destra o sopra)**: informazioni chat, pulsanti azione (Aggiungi membro, Leave, Impostazioni chat).

> Suggerimento screenshot: inserire qui uno screenshot della Home che mostri la sidebar, l’area messaggi e l’header. (File suggerito: `docs/screenshots/home.png`)

### Schermata Home
- Mostra elenco chat con anteprima dell’ultimo messaggio e timestamp
- Pulsante per creare nuova chat
- Campo di ricerca per trovare chat o contatti

### Lista contatti
- Cerca utenti per username
- Azioni: visualizza profilo, invia invito, inizia nuova chat

### Finestra chat
- **Header**: titolo chat, membri, informazioni
- **Area Messaggi**: visualizza messaggi in ordine cronologico; i messaggi mostrano nome mittente, testo e timestamp
- **Input**: campo testo, invio con Enter; tasti/icone per emoji o attach (se presenti)

> Suggerimento screenshot: finestra chat con messaggi e campo input (`docs/screenshots/chat_window.png`).

### Impostazioni
- Profilo utente: cambia username, immagine, logout
- Notifiche: abilitare/disabilitare notifiche browser
- Preferenze: temi, suoni (se implementati)

---

## 5. Come utilizzare la Chat

### Inviare e ricevere messaggi
- Per inviare un messaggio: seleziona la chat, digita nel campo, premi Enter.
- Il client invia un oggetto JSON (MessageDTO) via WebSocket. Il server persiste il messaggio e lo broadcast agli utenti connessi.
- Se sei offline, i messaggi verranno memorizzati e recapitati alla prossima connessione.

### Creare gruppi
1. Clic su `Nuova chat` → seleziona `Gruppo`.
2. Inserisci titolo e descrizione.
3. Aggiungi membri (seleziona dagli utenti o inserisci username).
4. Conferma per creare il gruppo; gli invitati riceveranno notifiche.

> Suggerimento screenshot: modal di creazione gruppo (`docs/screenshots/create_group.png`).

### Gestire le conversazioni (tutte le opzioni)
- **Aggiungere membro**: vai su dettagli chat → `Aggiungi membro` → seleziona utente → invia invito.
- **Rimuovere membro**: (solo OWNER/ADMIN) → dettagli chat → `Rimuovi membro`.
- **Cambiare ruolo**: (OWNER) → imposta ruolo ADMIN/MEMBER per gli utenti.
- **Trasferire ownership**: (OWNER) → `Transfer ownership` → seleziona nuovo owner.
- **Abbandonare chat**: `Leave chat` dal menu della chat.
- **Pulire chat**: (OWNER/ADMIN) → `Clean chat` per rimuovere messaggi secondo policy (azione distruttiva — richiede conferma).
- **Inviti**: gli inviti pendenti sono visibili in `/invitations/pending`; puoi accettare/rifiutare.

### Notifiche
- Notifiche in‑app: badge nella sidebar e nella chat.
- Notifiche browser: richiedere permesso; appaiono anche se la finestra non è attiva.
- Tipi di evento notificati: nuovo messaggio, invito, aggiunta a chat.

---

## 6. Sicurezza e Privacy

### Password e autenticazione
- Le password vengono inviate al server su canale sicuro (HTTPS) durante registrazione/login.
- Sul server le password sono **hashate usando `bcrypt`** prima di essere salvate nel database — il server non memorizza password in chiaro.
- Dopo il login viene emesso un JWT che il client usa per autenticare le richieste successive (header `Authorization: Bearer <token>`).

### Conservazione dei dati utente
- I messaggi, le informazioni utenti e gli inviti sono persistiti in MySQL.
- Sono memorizzati: username, hash password, messaggi (content, sender_id, created_at), metadati chat e inviti.
- L’accesso al DB dovrebbe essere protetto (firewall, credenziali limitate, backup regolari).

### Pratiche consigliate
- Usa password forti e uniche
- Non salvare token JWT in localStorage se temi XSS; preferisci session storage o meccanismi sicuri
- Assicurati che in produzione il servizio sia esposto solo via HTTPS e dietro reverse proxy sicuro

---

## 7. FAQ e risoluzione problemi rapida

Q: Non ricevo messaggi in tempo reale — cosa controllo?
- Verifica che la connessione WebSocket sia stabilita (console di rete nel browser)
- Controlla eventuali errori di autenticazione (token scaduto)
- Verifica che il reverse proxy non stia bloccando l’Upgrade/Connection headers

Q: Come recupero la password?
- Al momento la procedura di reset non è descritta nel progetto; contatta l’amministratore per reimpostare manualmente o implementare una procedura di reset via email.

Q: I messaggi scompaiono quando ricreo la chat?
- Azioni come `Clean chat` rimuovono i messaggi in modo permanente. Verifica i permessi prima di eseguire.

---

## 8. Appendice: comandi rapidi e contatti

- Logout: Menu Profilo → `Logout`.
- Eseguire un build locale (developer):
```pwsh
cd server
cargo build --release
```
- Test di integrazione (esempio):
```pwsh
cd server
cargo test --test api_websocket -- --nocapture
```

Contatti per assistenza: fornisci link o email dell’amministratore/maintainer del progetto.

---

## Posizionamento suggerito per screenshot
- Home: `docs/screenshots/home.png`
- Lista contatti: `docs/screenshots/contacts.png`
- Finestra chat: `docs/screenshots/chat_window.png`
- Creazione gruppo: `docs/screenshots/create_group.png`
- Notifica invito: `docs/screenshots/invitation.png`
- Impostazioni profilo: `docs/screenshots/settings.png`

Inserire le immagini con didascalie brevi e alt text per accessibilità.
