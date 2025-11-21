# Ruggine — Documentazione Tecnica



## 1. Indice completo


1. Indice completo
2. Panoramica del progetto
3. Tecnologie
4. Installazione
5. Configurazione
6. Avvio
7. Architettura del Sistema
  - Diagramma generale
  - Descrizione dei layer
  - Comunicazioni HTTP/WS
8. Architettura WebSocket (sezione dedicata)
  - Analisi approfondita `connection.rs`
  - Analisi approfondita `chatmap.rs`
  - Task di lettura/scrittura
  - Segnali interni
  - Gestione utenti
  - Messaggi e formati
  - Esempi dettagliati
9. Struttura del Progetto
  - Albero directory aggiornato
  - Descrizione responsabilità layer
10. API Documentation
  - Introduzione generale
  - Meccanismi di autenticazione
  - Endpoint Catalog (formato uniforme)
11. WebSocket Protocol Documentation
  - Endpoint
  - Lifecycle connessione
  - Eventi server → client
  - Eventi client → server
  - Errori, rate limiting, batching
12. Database Schema
  - Diagramma ER (ASCII)
  - Tabelle dettagliate (tipi, PK/FK, note)
  - Coerenza schema–modello
13. Test
   - Strategia
   - White tests
   - Test e2e
   - `sqlx` mocking
   - `axum` e2e
   - `tarpaulin` coverage report
14. Logging
   - Stack logging
   - Configurazioni
   - Esempi
15. Deployment Diagram & Context Diagram
   - Diagramma ASCII
   - Context Diagram
   - Descrizione nodi
   - Ambiente produzione/dev
16. Documentazione Client
   - Architettura front-end
   - Descrizione di ogni pagina
   - Descrizione dei componenti principali
   - Interazioni con API e WS
17. Dimensione del Compilato
   - Misurazioni
   - Ottimizzazioni
18. Troubleshooting
19. Sicurezza
20. Performance


---

## 2. Panoramica del progetto

Ruggine è un'applicazione di chat in tempo reale composta da un backend Rust (Axum + sqlx) e un frontend React/TypeScript (Vite). Il server espone API REST per gestione utenti, chat e inviti e un endpoint WebSocket autenticato per la comunicazione real-time (messaggi, inviti, notifiche). I messaggi sono persistiti su MySQL via `sqlx`. L'architettura privilegia separazione dei livelli (repositories/services/controllers) e un layer in-memory per il broadcasting via canali `tokio::sync::broadcast` gestiti da `ChatMap`.

Obiettivi principali:
- Comunicazione real-time efficiente (batching, broadcast)
- Persistenza sicura dei messaggi
- Middleware per autenticazione e membership
- Test coverage e strumenti di analisi

---

## 3. Tecnologie

Breve elenco delle tecnologie core usate nel server:

- Rust >= 1.70, runtime `tokio`
- Web framework: `axum`
- Database: MySQL con `sqlx`
- Serializzazione: `serde`
- Autenticazione: JWT (`jsonwebtoken`)
- Hashing password: `bcrypt`
- Concorrenza memorizzata: `dashmap`, `tokio::sync::broadcast`
- Logging: `tracing`, `tracing_subscriber`

## 4. Installazione

Prerequisiti:

- Rust (toolchain stabile / 1.70+)
- MySQL 8.0+
- Node.js + npm (per il client)

Setup rapido (server):

```pwsh
#clone del repository
git clone https://github.com/PdS2425-C2/G43.git
cd G43/server

#Installazione dipendeze
cargo build
```

Setup database:

```pwsh
# Avviare MySQL
mysql -u root -p

# Eseguire lo script di migrazione
source migrations/1_create_database.sql
```


## 5. Configurazione

Creare un file `.env` nella root `server/` con le variabili minime:


```env
# Database Configuration
DATABASE_URL=mysql://ruggine:ferro@127.0.0.1:3306/rugginedb

# Server Configuration
SERVER_HOST=127.0.0.1
SERVER_PORT=3000

# Database Pool Configuration
MAX_DB_CONNECTIONS=1000
DB_CONNECTION_LIFETIME_SECS=1

# Security
JWT_SECRET=your_super_secret_jwt_key_here

# Environment
APP_ENV=development
LOG_LEVEL=info
```

### Variabili d'Ambiente

| Variabile | Default | Descrizione |
|-----------|---------|-------------|
| `DATABASE_URL` | *(required)* | Connection string MySQL |
| `JWT_SECRET` | *(warning se mancante)* | Secret key per JWT |
| `SERVER_HOST` | `127.0.0.1` | Indirizzo di binding |
| `SERVER_PORT` | `3000` | Porta del server |
| `MAX_DB_CONNECTIONS` | `1000` | Max connessioni pool |
| `DB_CONNECTION_LIFETIME_SECS` | `1` | Durata connessioni pool |
| `APP_ENV` | `development` | Ambiente applicazione |
| `LOG_LEVEL` | `info` | Livello di logging |

## 6. Avvio

Development:

```pwsh
cd server
cargo run
```

Release:

```pwsh
cd server
cargo build --release
./target/release/server
```

Con hot-reload:

```pwsh
cargo install cargo-watch
cargo watch -x run
```

Output di Avvio:

```
Attempting to connect to database...
✓ Database connection established successfully!

Config {
    database_url: "mysql://ruggine:***@127.0.0.1:3306/rugginedb",
    server_host: "127.0.0.1",
    server_port: 3000,
    max_connections: 1000,
    connection_lifetime_secs: 1,
    app_env: "development",
    log_level: "info",
}

Server listening on http://127.0.0.1:3000
```
---

## 7. Architettura del Sistema

### Diagramma generale

```
+----------------+        HTTPS/API        +--------------------+
|   Client (UI)  | <---------------------> |  Reverse Proxy /   |
| React + WS     |                          |  Load Balancer     |
+----------------+        WebSocket         +--------------------+
        |                                         |
        | HTTP/WS                                  | Forward
        v                                         v
+----------------------+                   +-----------------------+
|  Server (Axum, Rust) |                   |  Database (MySQL)     |
| - REST API           |                   | - messages, users,    |
| - WebSocket endpoint |                   |   chats, invitations  |
| - ChatMap, UserMap   |                   +-----------------------+
| - Services / Repos   |
+----------------------+                   (optional) Monitoring/Logs
        |
        v
+----------------+
|  Background    |
|  Tasks:        |
|  - CPU monitor | 
|  - Cleanup     |
+----------------+
```

### Descrizione dei layer

- Transport layer: HTTP/HTTPS per API REST (Axum), WebSocket upgrade per canale real-time (`/ws`).
- Application layer: Handler Axum che orchestrano autenticazione + chiamate a services e repository.
- Services: Logica di business (inviti, messaggi, membership). Validano, trasformano DTO ↔ Entities.
- Repositories: Accesso persistente via `sqlx` su MySQL.
- Real-time layer: `ChatMap` (broadcast channels per chat) + `UserMap` (mappa utenti online) + per-connection tasks (read/write).
- Persistence: MySQL con tabelle `users`, `chats`, `messages`, `invitations`, `userchatmetadata`.

### Comunicazioni HTTP/WS

- HTTP: REST endpoints per login/register, gestione utenti, chat, inviti.
- WS: Endpoint `/ws` con autenticazione JWT (middleware) che esegue upgrade e poi `handle_socket` per split reader/writer.

---

## 8. Architettura WebSocket

Questa sezione descrive in dettaglio il comportamento interno del modulo WebSocket (`server/src/ws`).

Grafo funzionale (flow) dell'architettura WebSocket — mostra i componenti e il flusso dati/controllo:

```
Client (Browser)                           Reverse Proxy
     |                                         |
     |  HTTP Upgrade / Authorization Bearer    |
     |---------------------------------------->|
     |                                         |  forward
     |                                         v
     |                                  App Server (Axum)
     |                                         |
     |            WebSocket Upgrade (/ws)      |
     |<--------------------------------------->|
     |                                         |
     v                                         v
 +----------------+   spawn per-connection   +-------------------+
 |  listen_ws     |<------------------------>|   write_ws        |
 | (reader task)  |                          | (writer task)     |
 +-------+--------+                          +-----+-------------+
     |                                         |
     | on text msg                              | sends batches
     v                                         v
   event_handlers::process_message           BroadcastStream from ChatMap
     |                                         |
     v                                         v
   services/repositories -> persist message    ChatMap.send(chat_id, Arc<MessageDTO>)
     |                                         |
     v                                         v
   If user in chat -> OK                     Receivers (other write_ws tasks)
     |                                         |
     +-----------------------------------------+
          broadcast
```

Il grafo evidenzia i ruoli principali: client richiede upgrade (autenticato), il server crea due task per connessione (`listen_ws` e `write_ws`), `process_message` valida e persiste i messaggi e `ChatMap` si occupa del broadcasting ai writer task degli utenti connessi.

### 8.1 Analisi approfondita `connection.rs`

Ruolo: orchestrare la vita di una singola connessione WebSocket per utente autenticato. Le funzioni principali:
- `handle_socket(ws: WebSocket, state: Arc<AppState>, user_id: i32)` — entry point per ogni connessione.
- `write_ws(user_id, websocket_tx, internal_rx, state)` — task che invia dati al client.
- `listen_ws(user_id, websocket_rx, internal_tx, state)` — task che riceve messaggi dal client e li elabora.

Flusso generale di `handle_socket`:
1. Split della connessione (`ws.split()`) in `ws_tx` e `ws_rx`.
2. Creazione di un canale interno non-bounded (`unbounded_channel::<InternalSignal>()`) per ricevere segnali dall'applicazione a questa connessione.
3. Registrazione dell'utente come online nello `UserMap` via `state.users_online.register_online(user_id, int_tx.clone())`.
4. Spawn di due task concorrenti:
   - `listen_ws(...)`: legge messaggi dal socket, effettua rate-limiting e timeout, deserializza in `MessageDTO` e chiama `process_message`.
   - `write_ws(...)`: aggrega ricezioni dai canali broadcast (per le chat dell'utente), gestisce batching e segnali interni.

Assunzioni implementative e parametri:
- `RATE_LIMITER_MILLIS` (~10 ms) limita la frequenza di lettura (prevent brute force).
- `TIMEOUT_DURATION_SECONDS` (~300 s) chiude la connessione se inattiva.
- Batching: `BATCH_INTERVAL` (1000 ms) e `BATCH_MAX_SIZE` (10) — invio in batch verso client per efficienza.
- `BROADCAST_CHANNEL_CAPACITY` controlla capienza canali broadcast.

Cleanup e lifecycle:
- Quando `listen_ws` rileva Close, timeout o errore, invia `InternalSignal::Shutdown` al writer.
- `write_ws` risponde a `Shutdown` inviando batch finale e terminando.
- Alla terminazione, l'utente viene rimosso da `UserMap` con `state.users_online.remove_from_online(&user_id)`.

### 8.2 Analisi approfondita `chatmap.rs`

Ruolo: mantenere una mappa thread-safe (`DashMap<i32, Sender<Arc<MessageDTO>>>`) che associa `chat_id` → `broadcast::Sender<Arc<MessageDTO>>`.

Metodi chiave:
- `new()` — crea `ChatMap` con `DashMap` vuota.
- `subscribe(&self, chat_id: &i32) -> Receiver<Arc<MessageDTO>>` — se il canale esiste, ritorna un receiver; altrimenti crea un nuovo channel broadcast e lo inserisce.
- `subscribe_multiple(chat_ids: Vec<i32>) -> Vec<Receiver<Arc<MessageDTO>>>` — sottoscrive multiple chat e ritorna i receivers.
- `send(&self, chat_id: &i32, msg: Arc<MessageDTO>) -> Result<usize, SendError<Arc<MessageDTO>>>` — invia un messaggio al canale se esiste; se non ci sono ricevitori attivi rimuove il canale.

Comportamento e trade-offs:
- Uso di `Arc<MessageDTO>` evita copie costose tra sender e più receivers.
- Se `send` fallisce perché nessun receiver, il canale viene rimosso per evitare leak di canali senza ascoltatori.
- `subscribe` crea canali on-demand il che è efficiente per chat nuove ma può creare canali temporanei.

### 8.3 Task di lettura/scrittura

- Lettura (`listen_ws`): loop con `timeout(timeout_duration, StreamExt::next(&mut websocket_rx))`. Per ogni `Message::Text` viene tentata deserializzazione in `MessageDTO`. Se valida, si invoca `process_message`.
  - Validazioni: conversione in `CreateMessageDTO`, `validator::Validate`, matching `sender_id == user_id`.
  - In caso di invalidità o tentativi di spoofing, si invia `InternalSignal::Error` all'utente (via `users_online.send_server_message_if_online`).

- Scrittura (`write_ws`): costruisce un `StreamMap` contenente i receivers delle chat a cui l'utente è iscritto. Riceve messaggi tramite il broadcast channel e li accumula in `batch`. Il batch viene inviato quando:
  - la dimensione raggiunge `BATCH_MAX_SIZE`, o
  - l'`interval` (BATCH_INTERVAL) scatta.

- `internal_rx` (canale unbounded) permette di ricevere segnali interni:
  - `Shutdown`: fermare la connessione
  - `AddChat(chat_id)`: sottoscrivere una nuova chat e inviare notifica al client
  - `RemoveChat(chat_id)`: rimuovere la sottoscrizione e notificare
  - `Error(err_msg)`: inviare un messaggio di errore
  - `Invitation(payload)`: inviare inviti arricchiti

### 8.4 Segnali interni

Segnali definiti in `usermap.rs` (`InternalSignal`): `Shutdown`, `AddChat(i32)`, `RemoveChat(i32)`, `Error(&'static str)`, `Invitation(EnrichedInvitationDTO)`.

- L'uso di segnali consente al resto dell'app (services, repository) di comunicare rapidamente con connessioni attive.
- `UserMap::send_server_message_if_online` esegue lookup e invio sul `UnboundedSender` dell'utente se online.

### 8.5 Gestione utenti

- Registrazione: al momento dell'upgrade WS, `handle_socket` invoca `state.users_online.register_online(user_id, int_tx.clone())`.
- Rimozione: quando la connessione termina, `listen_ws` invia `Shutdown` e rimuove l'utente da `users_online`.
- `UserMap` mantiene un `DashMap<i32, UnboundedSender<InternalSignal>>` per lookup O(1).

### 8.6 Messaggi e formati

DTO usati come payload principale: `MessageDTO` (client ↔ server) e `CreateMessageDTO` interno a validazione.

`MessageDTO` (JSON):

```json
{
  "message_id": 123,        // optional
  "chat_id": 10,           // optional (ma necessario per Create)
  "sender_id": 42,         // optional (ma necessario per Create)
  "content": "ciao",
  "message_type": "UserMessage", // enum: UserMessage | SystemMessage
  "created_at": "2025-11-19T12:34:56Z"
}
```

- Client → Server (invio messaggio): inviare `MessageDTO` che contenga almeno `chat_id`, `sender_id`, `content`, `message_type`, `created_at`. Viene poi convertito a `CreateMessageDTO` per validazione.
- Server → Client (broadcast / batch): invio di array JSON di `MessageDTO` (batch), esempio:

```json
[
  {"message_id":1,"chat_id":10,"sender_id":2,"content":"Hi","message_type":"UserMessage","created_at":"..."},
  {"message_id":2,...}
]
```

- Segnali interni / notifiche: il writer invia JSON con chiave semantica, ad esempio:
  - `{"AddChat": 123}`
  - `{"RemoveChat": 123}`
  - `{"Invitation": { ... EnrichedInvitationDTO ... }}`
  - `{"Error": "Malformed message."}`

### 8.7 Esempi dettagliati

Esempio: flusso invio messaggio da client
1. Client invia (via WS): JSON `MessageDTO` con campi necessari.
2. Server (listen_ws) deserializza → `MessageDTO` → `CreateMessageDTO` (try_from).
3. `event_handlers::process_message` verifica membership usando `state.meta.read((user_id, chat_id))`.
4. `chats_online.send(&chat_id, Arc::from(msg))` per broadcasting ai receivers online.
5. Persistenza: `state.msg.create(&input_message)` salva message su DB.
6. Writer tasks dei client connessi (chat subscribers) ricevono il messaggio via `BroadcastStream` e lo includono nei batch per invio.

Esempio: server notifica invito
- Un servizio crea una `EnrichedInvitationDTO` e invoca `users_online.send_server_message_if_online(&target_user_id, InternalSignal::Invitation(inv))`.
- Se l'utente è online, il writer riceve il segnale e invia `{"Invitation": {...}}` al client.

---

## 9. Struttura del Progetto

### Albero directory 

```
/ (root)
├─ client/                      # Frontend React + Vite (TypeScript)
│  ├─ src/
│  │  ├─ components/            # UI components
│  │  ├─ context/               # Auth + WebSocket contexts
│  │  ├─ pages/                 # Login, Home
│  │  └─ services/              # api.ts, tauri.ts
│  └─ vite.config.ts
├─ server/                      # Backend Rust (Axum)
│  ├─ src/
│  │  ├─ core/                  # AppState, config, middleware
│  │  ├─ dtos/                  # DTOs per API / WS
│  │  ├─ entities/              # Domain entities (User, Chat, Message...)
│  │  ├─ repositories/          # DB access (sqlx)
│  │  ├─ services/              # Business logic
│  │  └─ ws/                    # WebSocket implementation (chatmap, connection...)
│  └─ migrations/
└─ docs/
```

### Descrizione responsabilità layer

- `core`: definisce `AppState` (pool DB, mappe in-memory), middleware (authentication, membership) e config.
- `dtos`: oggetti scambiati con client (HTTP + WS).
- `entities`: modelli interni coerenti con DB.
- `repositories`: tutte le query e transazioni `sqlx`.
- `services`: orchestrano logica, chiamano repositories e notificano utenti via `UserMap`/`ChatMap`.
- `ws`: implementazione real-time con `ChatMap` e `UserMap`.

---

## 10. API Documentation

### Introduzione generale

- Tutte le rotte sono montate in `server/src/main.rs`.
- Endpoint principali: `/auth/*`, `/users/*`, `/chats/*`, `/invitations/*`.
- Autenticazione: JWT (middleware `authentication_middleware`). I token sono presenti nell'header `Authorization: Bearer <token>`.

### Meccanismi di autenticazione

- Login restituisce JWT.
- Middleware `authentication_middleware` estrae e valida token e inserisce `Extension(User)` nei handler (usato anche per upgrade WS).

### Endpoint Catalog (formato uniforme)

Nota: Bodies e DTO si riferiscono a `server/src/dtos/*`.

### POST /auth/login
- URL: `/auth/login`
- HTTP Method: POST
- Protetta: No
- Description: Effettua login e restituisce JWT.
- Path parameters: None
- Query parameters: None
- Request body: `{ "username": "string", "password": "string" }`
- Response status: 200 OK / 401 Unauthorized
- Response body:

```json
{
  "token": "<jwt>",
  "user": { "user_id": 1, "username": "mario_rossi" }
}
```

---

### POST /auth/register
- URL: `/auth/register`
- HTTP Method: POST
- Protetta: No
- Description: Registra nuovo utente.
- Path parameters: None
- Query parameters: None
- Request body:

```json
{ "username": "mario_rossi", "password": "SecurePass123!" }
```
- Response status: 201 Created / 400 Bad Request
- Response body:

```json
{ "user_id": 1, "username": "mario_rossi" }
```

---

### GET /users
- URL: `/users/`
- HTTP Method: GET
- Protetta: Sì
- Description: Cerca utenti per username (query param)
- Path parameters: None
- Query parameters: `username` (string, partial search)
- Request body: None
- Response status: 200 OK
- Response body:

```json
[
  { "user_id": 1, "username": "mario_rossi" },
  { "user_id": 5, "username": "mario_bianchi" }
]
```

---

### GET /users/me
- URL: `/users/me`
- HTTP Method: GET
- Protetta: Sì
- Description: Ottiene informazioni sull'utente autenticato.
- Path parameters: None
- Query parameters: None
- Request body: None
- Response status: 200 OK
- Response body:

```json
{ "user_id": 1, "username": "mario_rossi" }
```

---

### GET /users/{user_id}
- URL: `/users/{user_id}`
- HTTP Method: GET
- Protetta: Sì
- Description: Recupera utente per `user_id`.
- Path parameters: `user_id` (int)
- Query parameters: None
- Request body: None
- Response status: 200 OK / 404 Not Found
- Response body:

```json
{ "user_id": 1, "username": "mario_rossi" }
```

---

### GET /chats
- URL: `/chats/`
- HTTP Method: GET
- Protetta: Sì
- Description: Lista chat dell'utente (con metadati come ruolo e unread)
- Request body: None
- Response status: 200 OK
- Response body:

```json
[
  { "chat_id": 1, "title": "Team Project", "description": "Progetto università", "chat_type": "GROUP", "my_role": "OWNER", "unread_messages": 5, "last_message_at": "2025-11-05T14:30:00Z" }
]
```

---

### POST /chats
- URL: `/chats/`
- HTTP Method: POST
- Protetta: Sì
- Description: Crea nuova chat
- Request body (Group):

```json
{ "title": "New Project Group", "description": "Chat per il nuovo progetto", "chat_type": "GROUP" }
```

- Request body (Private):

```json
{ "chat_type": "PRIVATE", "other_user_id": 5 }
```
- Response status: 201 Created
- Response body:

```json
{ "chat_id": 3, "title": "New Project Group", "description": "Chat per il nuovo progetto", "chat_type": "GROUP" }
```

---

### GET /chats/{chat_id}/messages
- URL: `/chats/{chat_id}/messages`
- HTTP Method: GET
- Protetta: Sì (membership)
- Description: Recupera messaggi di una chat (paginati)
- Path parameters: `chat_id` (int)
- Query parameters: `limit`, `before`, `after`
- Request body: None
- Response status: 200 OK
- Response body:

```json
[
  { "message_id": 10, "chat_id": 1, "sender_id": 2, "content": "Hi", "message_type": "USERMESSAGE", "created_at": "2025-11-05T14:00:00Z" }
]
```

---

### GET /chats/{chat_id}/members
- URL: `/chats/{chat_id}/members`
- HTTP Method: GET
- Protetta: Sì (membership)
- Description: Lista membri della chat
- Response status: 200 OK
- Response body:

```json
[
  { "user_id": 1, "username": "mario_rossi", "user_role": "OWNER" }
]
```

---

### POST /chats/{chat_id}/invite/{user_id}
- URL: `/chats/{chat_id}/invite/{user_id}`
- HTTP Method: POST
- Protetta: Sì (membership)
- Description: Invia invito a `user_id` a unirsi alla chat
- Path parameters: `chat_id`, `user_id`
- Request body: None
- Response status: 200 OK / 404 Not Found
- Response body (example EnrichedInvitationDTO):

```json
{
  "invite_id": 10,
  "target_chat_id": 1,
  "invited_id": 5,
  "invitee_id": 1,
  "state": "PENDING",
  "created_at": "2025-11-05T15:00:00Z"
}
```

Note: se il target è online, il server invia un `InternalSignal::Invitation` via `UserMap`.

---

### PATCH /chats/{chat_id}/members/{user_id}/role
- URL: `/chats/{chat_id}/members/{user_id}/role`
- HTTP Method: PATCH
- Protetta: Sì (membership)
- Description: Aggiorna ruolo membro (OWNER|ADMIN|MEMBER)
- Request body:

```json
{ "user_role": "ADMIN" }
```
- Response status: 200 OK

---

### PATCH /chats/{chat_id}/transfer_ownership/{new_owner_id}
- URL: `/chats/{chat_id}/transfer_ownership/{new_owner_id}`
- HTTP Method: PATCH
- Protetta: Sì
- Description: Trasferisce ownership della chat
- Response status: 200 OK

---

### DELETE /chats/{chat_id}/members/{user_id}
- URL: `/chats/{chat_id}/members/{user_id}`
- HTTP Method: DELETE
- Protetta: Sì
- Description: Rimuove membro dalla chat
- Response status: 204 No Content

---

### POST /chats/{chat_id}/leave
- URL: `/chats/{chat_id}/leave`
- HTTP Method: POST
- Protetta: Sì
- Description: L'utente autenticato lascia la chat
- Response status: 200 OK

---

### POST /chats/{chat_id}/clean
- URL: `/chats/{chat_id}/clean`
- HTTP Method: POST
- Protetta: Sì
- Description: Pulisce la chat (admin/owner)
- Response status: 200 OK

---

### GET /invitations/pending
- URL: `/invitations/pending`
- HTTP Method: GET
- Protetta: Sì
- Description: Lista inviti pendenti per utente
- Response status: 200 OK
- Response body (example array):

```json
[
  { "invite_id": 10, "target_chat_id": 1, "invited_id": 5, "invitee_id": 1, "state": "PENDING", "created_at": "2025-11-05T15:00:00Z" }
]
```

---

### POST /invitations/{invite_id}/{action}
- URL: `/invitations/{invite_id}/{action}`
- HTTP Method: POST
- Protetta: Sì
- Description: Rispondi a invito; `action` = `accept|reject`
- Path params: `invite_id`, `action`
- Response status: 200 OK

---

### WebSocket endpoint: /ws
- URL: `/ws`
- HTTP Method: GET (upgrade WebSocket)
- Protetta: Sì (middleware `authentication_middleware`)
- Description: Upgrade autenticato a connessione WebSocket per ricevere/send messaggi real-time.

Note generali:
- Tutte le rotte marchiate come protette richiedono header `Authorization: Bearer <token>`.
- I DTO sono definiti in `server/src/dtos`.

---

## 11. WebSocket Protocol Documentation

### Endpoint

- `/ws` — upgrade dal client autenticato. Il middleware inserisce `Extension(User)` per `user_id` usato da `handle_socket`.

### Lifecycle connessione

1. Client richiede upgrade WS a `/ws` con token.
2. Server verifica il JWT (middleware) e recupera `User`.
3. `ws_handler` esegue upgrade e chiama `handle_socket(socket, state, user_id)`.
4. `handle_socket` crea `internal_channel`, registra l'utente e avvia `listen_ws` e `write_ws`.
5. Durante la vita della connessione: client invia `MessageDTO` → server elabora; server invia batch di `MessageDTO` e notifiche.
6. Alla chiusura o timeout, `Shutdown` e rimozione utente da `UserMap`.

### Eventi server → client

- `Batch Messages` (array di `MessageDTO`) — invio periodico o a batch_size.
- `AddChat` / `RemoveChat` — notifiche con forma `{"AddChat": chat_id}`.
- `Invitation` — `{"Invitation": EnrichedInvitationDTO}`.
- `Error` — `{"Error": "message"}`.

Esempio JSON batch:

```json
[   
    {"message_id":1,...}, 
    {"message_id":2,...} 
]
```

### Eventi client → server

- `MessageDTO` — invio messaggi. Il server aspetta campi necessari per creare `CreateMessageDTO` (`chat_id`, `sender_id`, `content`, `message_type`, `created_at`).

Esempio client→server:

```json
{
    "chat_id":10,
    "sender_id":42,
    "content":"Ciao",
    "message_type":"UserMessage","created_at":"2025-11-19T12:34:56Z"
}
```

### Errori, rate limiting, batching

- Rate limiting lato server: `RATE_LIMITER_MILLIS` (10 ms) → limite pratico ~100 msg/s per connessione.
- Timeout inattività: `TIMEOUT_DURATION_SECONDS` (300s) → chiusura automatica.
- Batching: `BATCH_INTERVAL` (1000 ms) e `BATCH_MAX_SIZE` (10) per ridurre overhead di invio.
- Error handling: invalid message → `InternalSignal::Error` notificato al client; tentativi di spoofing o violazioni → rejection e log.
- Se il channel broadcast non ha receivers, `ChatMap::send` ritorna errore e il messaggio viene comunque persistito sul DB per consegna successiva.

---

## 12. Database Schema

### Diagramma ER

```
+--------+      +-----------------+      +----------+
| users  |<---->| userchatmetadata|<---->| chats    |
| (PK)   |      | (PK: chat_id,   |      | (PK)     |
| user_id|      |       user_id)  |      | chat_id  |
+--------+      +-----------------+      +----------+
     |                |  ^   ^               |
     |                |  |   |               |
     |                |  |   +-----------+   |
     |                |  |               |   |
     +--< messages >--+  +--< invitations >--+

Tables:
- users
- chats
- messages
- invitations
- userchatmetadata
```

### Tabelle dettagliate

1) `users`
- `user_id` INT PK AUTO_INCREMENT
- `username` VARCHAR(255) UNIQUE NOT NULL
- `password` TEXT NOT NULL (bcrypt hashed)

2) `chats`
- `chat_id` INT PK AUTO_INCREMENT
- `title` VARCHAR(255)
- `description` TEXT
- `chat_type` ENUM('GROUP','PRIVATE') NOT NULL

3) `messages`
- `message_id` INT PK AUTO_INCREMENT
- `chat_id` INT FK -> `chats.chat_id` ON DELETE CASCADE
- `sender_id` INT FK -> `users.user_id` ON DELETE CASCADE
- `content` TEXT NOT NULL
- `message_type` ENUM('USERMESSAGE','SYSTEMMESSAGE')
- `created_at` TIMESTAMP NOT NULL
- Indici: `(chat_id, created_at DESC)`, `(sender_id)`

4) `invitations`
- `invite_id` INT PK AUTO_INCREMENT
- `target_chat_id` INT FK -> `chats.chat_id`
- `invited_id` INT FK -> `users.user_id`
- `invitee_id` INT FK -> `users.user_id`
- `state` ENUM('PENDING','ACCEPTED','REJECTED')
- `created_at` TIMESTAMP
- Unique constraint: `(target_chat_id, invited_id, state)`

5) `userchatmetadata`
- PK (`chat_id`,`user_id`)
- `messages_visible_from` TIMESTAMP NOT NULL
- `messages_received_until` TIMESTAMP NOT NULL
- `user_role` ENUM('OWNER','ADMIN','MEMBER')
- `member_since` TIMESTAMP NOT NULL



---

---

## 13. Test

### Strategia

- Test unitari (white tests) per repository e middleware (autenticazione, membership).
- Test di integrazione/e2e su servizi critici (services + repository) usando `axum` test utilities e un database di test (fixture SQL o container). Uso di `sqlx` per interagire con DB; in test si può usare `sqlx::Sqlite` o MySQL in-memory/isolato.
- Report di coverage con `tarpaulin` (per Rust) e strumenti JS per client.

### White tests (unit)

- Repositories: testare le query SQL in isolamento usando `sqlx::test` e fixture su DB di test.
- Middleware: testare comportamento di `authentication_middleware` (header mancanti, token invalidi) con request fittizie.

### Test e2e

- Avviare `axum` in tokio test runtime e chiamare endpoint reali (es. login -> create chat -> invite -> websocket upgrade).
- Per WS, usare client WebSocket di test (ad esempio `tokio-tungstenite`) per simulare upgrade e verificare flussi di messaggi end-to-end.

### `sqlx` mocking

- `sqlx` permette di usare un database reale nelle suite di test, oppure usare feature `offline` con query compile-time check e database di test.
- Fixtures SQL si trovano in `server/fixtures/` (users.sql, chats.sql, messages.sql, invitations.sql) e devono essere caricate prima dei test e2e.

### `axum` e2e

- Usare `axum::Router` con state di test e layer middleware identici a produzione; chiamare handlers con `tower::Service` o `reqwest` su listener TCP.



---

## 14. Logging

### Stack logging

- Libreria: `tracing` + `tracing_subscriber`.
- In `main.rs` si inizializza `tracing_subscriber::EnvFilter` con valore prelevato dalla configurazione (`RUST_LOG` o `config.log_level`).
- Layer: `fmt::layer()` per formatting di logs.

### Formati

- Default: plain text legibile.
- Possibile JSON: `tracing_subscriber::fmt().json()` per output strutturato in produzione e log shipping.

### Configurazioni

- Variabili d'ambiente: `RUST_LOG` (es. `server=info,tower_http=debug`) o `LOG_LEVEL` nel file di configurazione.
- Esempio avvio con livello debug:

```pwsh
$env:RUST_LOG = "server=debug,tower_http=debug"
cargo run --bin server
```

### Esempi di log significativi

- Connessione WS stabilita: `WebSocket connection established`
- User registrato online: `User registered as online`
- Messaggio broadcast: `Message broadcast to receivers`
- Errori DB: `Failed to persist message to database`

---

## 15. Deployment Diagram



```
 [Developer] -> CI/CD -> [Registry / Artifacts]
                            |
                            v
                   +-----------------------+
                   |  Reverse Proxy (nginx) |
                   +-----------------------+
                            |
                            v
    +-------------------+         +-------------------+
    | App Server (Rust) |  <---->  |  MySQL Database   |
    | (container/or VM) |         | (managed / RDS)   |
    +-------------------+         +-------------------+
             |
             v
       Optional: Metrics / Logs -> ELK / Prometheus
```
## 16. Context Diagram

```

        +-------------------+            +----------------+
        |    External       |            |   External     |
        |    Identity / SSO  |            | Monitoring /   |
        |    Provider       |            | Logging (ELK)  |
        +---------+---------+            +--------+-------+
            |                               |
            v                               v
          +--------------------------------+    +---------------------------+
          |  Reverse Proxy (nginx)         |    |  CI/CD / Registry         |
          |  (TLS, routing, compression)   |    |  (build images, artifacts)|
          +---------------+----------------+    +-------------+-------------+
              |                                   |
              v                                   v
            +-------------------+                 +-------------------+
            |  App Server (Rust)| <--------------> |  Database (MySQL) |
            |  (Axum + WS)      |                 |  (rugginedb)      |
            +---+------+--------+                 +-------------------+
          |      |
          |      +--> ChatMap / UserMap (in-memory)
          |
          +--> Background tasks (metrics, cpu-monitor)
```

### Descrizione nodi

- Client: browser con React app.
- Reverse Proxy: TLS termination, routing a `/` e `/ws` al server; può gestire scaling e sticky sessions se necessario.
- App Server: esegue il binario Rust; gestisce REST + WS.
- DB: MySQL (rugginedb) con connessioni pool.
- CI/CD: build artifacts, publish image, deploy.

### Pipeline di deploy

1. CI: run tests, build release (`cargo build --release`), build client (`npm run build`).
2. Package: creare Docker image (server + assets) o artefatto binario.
3. Publish: push su registry.
4. Deploy: rollout su ambiente (K8s/VM) + migrazioni DB.

---

## 17. Documentazione Client

### Architettura front-end

- Framework: React + TypeScript + Vite.
- State/Context: `AuthContext` per autenticazione; `WebSocketContext` per connessione WS condivisa.
- Servizi: `services/api.ts` per HTTP, `services/tauri.ts` per integrazione desktop/Tauri se attivo.

### Routing

- Pagine principali: `Login` (autenticazione), `Home` (lista chat + area chat), componenti chat.

### Descrizione di ogni pagina

- `Login`:
  - Cosa mostra: form username/password.
  - Azioni: POST `/auth/login` → salva JWT in context.
  - Interazione WS: dopo login, apre connessione WS a `/ws` con header `Authorization`.

- `Home`:
  - Cosa mostra: sidebar con chat, area messaggi, header chat.
  - Azioni: selezione chat, invio messaggi via WebSocket, gestione inviti.
  - Interazione API: richieste REST per liste chat e storico messaggi, WS per ricevere messaggi in tempo reale.

### Componenti principali 

- `Sidebar`: lista chat; interroga GET `/chats`.
- `ChatArea`: mostra messaggi (chiama GET `/chats/{chat_id}/messages`) e si registra alle notifiche WS.
- `ChatInput`: invia `MessageDTO` via WS al server.
- `ProfileModal`: gestione utente.

### Interazioni API e WS

- HTTP per operazioni CRUD, membership, inviti e fetch storici.
- WS usato per invio immediato di messaggi e ricezione broadcast / notifiche.

---

## 18. Dimensione del Compilato

### Misurazioni (ambiente attuale nella workspace)

- Binario server (debug) rilevato in `server/target/debug/server.exe`: 16.03 MB (misurazione locale).

Comandi usati per verificare:

```pwsh
Get-Item 'server/target/debug/server.exe' | Select-Object Name,@{Name='SizeMB';Expression={[math]::Round($_.Length/1MB,2)}}
```

### Misure consigliate da riprodurre

Per ottenere misure ripetibili:

1. Build debug
```pwsh
cd server
cargo build
Get-Item target/debug/server.exe | Select Name,@{Name='SizeMB';Expression={[math]::Round($_.Length/1MB,2)}}
```

2. Build release (ottimizzato)
```pwsh
cd server
cargo build --release
Get-Item target/release/server.exe | Select Name,@{Name='SizeMB';Expression={[math]::Round($_.Length/1MB,2)}}
```

3. Build client (Vite)
```pwsh
cd client
npm install
npm run build
# misura dist
Get-ChildItem -Recurse dist | Measure-Object -Property Length -Sum
```

### Ottimizzazioni consigliate

- Rust: `cargo build --release --features=...` + `LTO` (link-time-optimization) in `Cargo.toml` per ridurre dimensione e migliorare perf.
  - `strip` binario per rimuovere simboli: `strip target/release/server.exe` (o `llvm-strip`)
- Client: abilitare minification, tree-shaking (Vite/rollup), compressione asset (gzip/brotli) in server/reverse proxy.

---

## 19. Troubleshooting

- Problema: connessioni WS cadono subito -> verificare `Authorization` header, controllare logs per `Connection timeout` e `RATE_LIMITER_MILLIS`.
- Problema: messaggi non consegnati -> controllare `ChatMap::send` warning: se nessun receiver, il canale viene rimosso; i messaggi sono comunque persistiti.
- Problema: DB connection fail -> `main.rs` esegue retry in loop; verificare variabile `DATABASE_URL` nel `.env`.

---

## 20. Sicurezza

- Token JWT: validare scadenza e firma; usare secret robusto.
- Password: sempre hashare con `bcrypt` (già implementato).
- SQLi: `sqlx` con query parametrizzate evita injection.
- Minimizzare permessi DB: user `ruggine` con privilegi limitati (SELECT, INSERT, UPDATE, DELETE, CREATE, INDEX, ALTER).
- Rate limiting: almeno lato WS; considerare rate limit in ingress (reverse proxy) per protezione DoS.

---

## 21. Performance

- Batching WS riduce overhead di rete `BATCH_MAX_SIZE` e `BATCH_INTERVAL`.
- Uso di `Arc<MessageDTO>` evita copie multiple nella broadcast.
- `DashMap` e `tokio::sync::broadcast` offrono concorrenza efficiente.
- Consigli: abilitare `RUST_LOG` a livello info/warn in produzione; profilare con `perf`/`flamegraph` per hot-path.

---
