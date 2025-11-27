# Ruggine — Documentazione Tecnica



## Indice completo

1. [Panoramica del progetto](#1-panoramica-del-progetto)
2. [Requisiti Funzionali](#2-requisiti-funzionali)
3. [Requisiti Non Funzionali](#3-requisiti-non-funzionali)
4. [Librerie utilizzate](#4-librerie-utilizzate)
5. [Installazione](#5-installazione)
6. [Configurazione](#6-configurazione)
7. [Avvio](#7-avvio)
8. [Architettura del Sistema](#8-architettura-del-sistema)
9. [Architettura WebSocket](#9-architettura-websocket)
10. [Struttura del Progetto](#10-struttura-del-progetto)
11. [API Documentation](#11-api-documentation)
12. [WebSocket Protocol Documentation](#12-websocket-protocol-documentation)
13. [Database Schema](#13-database-schema)
14. [Test](#14-test)
15. [Logging](#15-logging)
16. [Deployment & Context Diagram](#16-deployment--context-diagram)
17. [Documentazione Client](#17-documentazione-client)
18. [Dimensione del Compilato](#18-dimensione-del-compilato)
19. [Sicurezza](#19-sicurezza)
20. [Performance](#20-performance)


---

## 1. Panoramica del progetto

**Ruggine** è un'applicazione desktop di chat in tempo reale sviluppata con:
- **Backend**: Rust con Axum 0.8.4 e sqlx 0.8.6 per persistenza MySQL
- **Frontend**: React 19.1.0 + TypeScript 5.8.3 impacchettato con Tauri 2.x

Il sistema espone API REST per operazioni CRUD (utenti, chat, inviti) e un endpoint WebSocket (`/ws`) per comunicazione real-time. L'architettura adotta una chiara separazione in layer:
- **Transport**: HTTP/HTTPS per REST, WebSocket per messaggistica real-time
- **Services**: Logica di business (auth, chat, membership)
- **Repositories**: Accesso dati con sqlx
- **Real-time**: UserMap (DashMap) + ChatMap (broadcast channels tokio)
- **Persistence**: MySQL 8.0+ con migrazioni sqlx

**Caratteristiche implementate**:
- Autenticazione JWT (expire 24h)
- Chat di gruppo e private
- Sistema di inviti con notifiche real-time
- Batching messaggi WebSocket (max 10 msg o 1 sec)
- Rate limiting client (10ms → ~100 msg/sec)
- Timeout inattività WebSocket (5 minuti)
- Monitoraggio CPU del processo server
- Client Tauri desktop con notifiche native

---

## 2. Requisiti Funzionali

Questa sezione documenta i requisiti funzionali effettivamente implementati nel progetto.

### 2.1 Gestione Utenti

**Autenticazione:**
- **Login** (`POST /auth/login`): Autenticazione tramite username e password, ritorna JWT (HS256, expire 24h)
- **Registrazione** (`POST /auth/register`): Creazione nuovo utente con hash bcrypt (cost 12)
- **Validazione**: Username riservato "Deleted User" bloccato in fase di registrazione/login

**Ricerca:**
- **Ricerca utente** (`GET /users?username={username}`): Filtro utenti tramite username (query parameter)
- **Lista utenti** (`GET /users`): Recupero di tutti gli utenti registrati

### 2.2 Gestione Chat Private

**Funzionalità implementate:**
- **Creazione chat privata**: Automatica quando si invia il primo invito tra due utenti (`POST /chats/{chat_id}/invite/{user_id}`)
- **Recupero messaggi** (`GET /chats/{chat_id}/messages`): Con filtro basato su `messages_visible_from` (ultimo messaggio visualizzato)
- **Salvataggio ultimo messaggio visualizzato**: Campo `messages_received_until` in `user_chat_metadata`
- **Invio messaggio**: Tramite WebSocket con validazione lunghezza (1-5000 caratteri)
- **Pulizia chat** (`POST /chats/{chat_id}/clean`): Aggiorna `messages_visible_from` ed elimina messaggi non più visibili da nessun membro

### 2.3 Gestione Chat di Gruppo

**Creazione e configurazione:**
- **Creazione gruppo** (`POST /chats`): Con `chat_type: "Group"`, `title`, `description` opzionale
- **Ruoli utente**: Owner, Admin, Member implementati con permessi differenziati

**Gestione messaggi:**
- **Recupero messaggi** (`GET /chats/{chat_id}/messages`): Con filtro `messages_visible_from`
- **Salvataggio ultimo messaggio visualizzato**: Campo `messages_received_until` in metadata
- **Invio messaggio**: WebSocket con validazione (1-5000 caratteri, rate limiting 10ms)
- **Pulizia messaggi per singolo utente** (`POST /chats/{chat_id}/clean`): Aggiorna `messages_visible_from`, elimina fisicamente messaggi non visibili da nessuno

**Gestione membri:**
- **Estensione ruolo Admin** (`PATCH /chats/{chat_id}/members/{user_id}/role`): Owner può promuovere Member → Admin
- **Aggiunta membri tramite invito** (`POST /chats/{chat_id}/invite/{user_id}`): Owner/Admin invitano, target riceve notifica real-time
- **Risposta invito** (`POST /invitations/{invite_id}/{action}`): Accept/Reject, crea messaggio di sistema
- **Lista inviti pending** (`GET /invitations/pending`): Inviti ricevuti dall'utente autenticato
- **Espulsione membro** (`DELETE /chats/{chat_id}/members/{user_id}`): Solo Owner/Admin, non può rimuovere Owner
- **Uscita spontanea** (`POST /chats/{chat_id}/leave`): Member/Admin possono uscire, Owner solo se unico membro
- **Trasferimento ownership** (`PATCH /chats/{chat_id}/transfer_ownership/{new_owner_id}`): Solo Owner, target deve essere membro

**Funzionalità real-time:**
- **Notifiche WebSocket**: AddChat, RemoveChat, Invitation inviati tramite `InternalSignal`
- **Broadcast messaggi**: Batching (10 msg o 1 sec), Arc<MessageDTO> zero-copy
- **Rate limiting**: 10ms per messaggio (~100 msg/sec per connessione)
- **Timeout inattività**: 300 secondi (5 minuti)

### 2.4 Logging

**Stack implementato:**
- **tracing 0.1** + **tracing-subscriber 0.3**
- **Configurazione livelli**: Via `RUST_LOG` env var (`trace`, `debug`, `info`, `warn`, `error`)
- **Integrazione tower-http**: Logging middleware per richieste HTTP
- **Structured logging**: Campi contestuali (`user_id`, `chat_id`, etc.) tramite macro `#[instrument]`

---

## 3. Requisiti Non Funzionali

### 3.1 Portabilità Multi-Piattaforma

**Cross-platform (Windows e Linux):**
- **Rust Edition 2024**: Linguaggio cross-platform nativo
- **Tokio 1.47.1**: Runtime async multi-piattaforma
- **Tauri 2.x**: Client desktop cross-platform (Windows, Linux, macOS)
- **Dipendenze**: Tutte le librerie supportano Windows/Linux (axum, sqlx, bcrypt, etc.)

### 3.2 Efficienza CPU e Dimensione Compilato

**Async runtime** (`server/Cargo.toml`):
```toml
tokio = { version = "1.47.1", features = ["rt-multi-thread", ...] }
```
- **M:N threading**: Migliaia di task async su pochi thread OS
- **Work-stealing scheduler**: Bilanciamento automatico carico
- **DashMap 6.1.0**: HashMap concorrente lock-free
- **Arc<MessageDTO>**: Zero-copy broadcasting

**Ottimizzazioni release** (`server/Cargo.toml`):
```toml
[profile.release]
opt-level = "z"        # Ottimizza per size
lto = true             # Link-Time Optimization
codegen-units = 1      # Ottimizzazioni globali
panic = "abort"        # Riduce size ~10%
strip = true           # Rimuove simboli debug (~30% reduction)
```
---

## 4. Librerie utilizzate

### Backend (Rust)
**Framework e Runtime:**
- Axum 0.8.4 (web framework + WebSocket)
- Tokio 1.47.1 (async runtime multi-thread)
- Axum-macros 0.5.0

**Database:**
- sqlx 0.8.6 (MySQL pool, query compile-time checked)
- MySQL 8.0.43+

**Autenticazione e Sicurezza:**
- bcrypt 0.17.1 (hash password, cost=12)
- jsonwebtoken 9.3.1 (JWT HS256)

**Serializzazione:**
- serde 1.0.226, serde_json 1.0.145
- chrono 0.4.42 (timestamp)

**Validazione:**
- validator 0.18 (derive macro per DTO)

**Concorrenza:**
- dashmap 6.1.0 (HashMap thread-safe)
- tokio::sync::broadcast (canali multi-producer multi-consumer)
- futures 0.3.31, tokio-stream 0.1.17

**Utilità:**
- tower-http 0.6 (CORS middleware)
- tracing 0.1, tracing-subscriber 0.3 (logging strutturato)
- sysinfo 0.32.1 (monitoraggio CPU/memoria)
- dotenv 0.15 (variabili ambiente)
- lazy_static 1.5, regex 1.11

**Dev/Test:**
- axum-test 18.1.0 (test HTTP)
- tokio-tungstenite 0.24.0 (test WebSocket)

### Frontend (React + Tauri)
**Framework UI:**
- React 19.1.0 + React DOM 19.1.0
- React Router DOM 7.9.5
- Bootstrap 5.3.8 + React Bootstrap 2.10.10
- Bootstrap Icons 1.13.1

**Build:**
- Vite 7.0.4
- TypeScript 5.8.3
- @vitejs/plugin-react 4.6.0

**Tauri Desktop:**
- @tauri-apps/api ^2
- @tauri-apps/cli 2.9.0
- @tauri-apps/plugin-notification 2.3.3
- @tauri-apps/plugin-opener ^2

**Backend Tauri (Rust):**
- tauri 2.x, tauri-build 2.x
- tokio-tungstenite 0.21 (WebSocket client)
- serde 1.x, serde_json 1.x

## 5. Installazione

### Prerequisiti

**Software richiesto:**
- Rust toolchain stabile (edition 2024)
- MySQL 8.0.43 o superiore
- Node.js 18+ e npm (per il client)
- Tauri CLI (installato automaticamente tramite npm)

### Setup Server

```pwsh
# Clone del repository
git clone https://github.com/PdS2425-C2/G43.git
cd G43/server

# Build dipendenze Rust
cargo build
```

### Setup Database

```pwsh
# Avvia MySQL server
mysql -u root -p

# Esegui script di migrazione completo
SOURCE migrations/1_create_database.sql;
```

Lo script crea:
- Database `rugginedb` (charset utf8mb4_unicode_ci)
- Utente `ruggine` con password `ferro`
- Tabelle: users, chats, messages, invitations, userchatmetadata
- Indici ottimizzati per query frequenti

### Setup Client Tauri

```pwsh
cd ../client

# Installa dipendenze npm
npm install

# Build frontend (opzionale per dev)
npm run tauri build
```


## 6. Configurazione

### File `.env` Server

Creare `server/.env` con le seguenti variabili:

```env
# Database Configuration (OBBLIGATORIO)
DATABASE_URL=mysql://ruggine:ferro@127.0.0.1:3306/rugginedb

# Security (opzionale, default insicuro)
JWT_SECRET=your_super_secret_jwt_key_here_min_32_chars

# Server Configuration (opzionali)
SERVER_HOST=127.0.0.1
SERVER_PORT=3000

# Database Pool (opzionali)
MAX_DB_CONNECTIONS=1000
DB_CONNECTION_LIFETIME_SECS=1

# Environment (opzionali)
APP_ENV=development
LOG_LEVEL=info
```

### Variabili d'Ambiente

| Variabile | Default | Obbligatorio | Descrizione |
|-----------|---------|--------------|-------------|
| `DATABASE_URL` | - | ✅ | Connection string MySQL formato: `mysql://user:password@host:port/database` |
| `JWT_SECRET` | `"supersecretkey"` | ❌ | Chiave firma JWT (HS256). **WARNING**: default non sicuro per produzione |
| `SERVER_HOST` | `127.0.0.1` | ❌ | Indirizzo IP di binding server |
| `SERVER_PORT` | `3000` | ❌ | Porta TCP server (0-65535) |
| `MAX_DB_CONNECTIONS` | `1000` | ❌ | Dimensione pool connessioni MySQL |
| `DB_CONNECTION_LIFETIME_SECS` | `1` | ❌ | Durata max connessione in secondi |
| `APP_ENV` | `development` | ❌ | Ambiente: development/production |
| `LOG_LEVEL` | `info` | ❌ | Livello log tracing: trace/debug/info/warn/error |

### Configurazione Client

Creare `client/.env` (opzionale):

```env
# WebSocket URL per connessione backend
VITE_WS_URL=ws://localhost:3000/ws

# API Base URL
VITE_API_URL=http://localhost:3000
```

**Note implementative:**
- `Config::from_env()` carica variabili tramite `dotenv` in `server/src/core/config.rs`
- JWT_SECRET default genera warning al boot: `"WARNING: JWT_SECRET not set, using default (not secure for production!)"`
- `Config::mask_url()` nasconde password in log output
- Acquire timeout pool: 2 secondi (hardcoded)
- Test before acquire: abilitato

## 7. Avvio

### Avvio Server (Backend)

**Development mode:**
```pwsh
cd server
cargo run
```

**Release optimized:**
```pwsh
cd server
cargo build --release
.\target\release\server.exe
```

**Con auto-reload (opzionale):**
```pwsh
cargo install cargo-watch
cargo watch -x run
```

**Output atteso al boot:**
```
   Server Configuration:
   Environment: development
   Log Level: info
   Server Address: 127.0.0.1:3000
   Database: mysql://***@127.0.0.1:3306/rugginedb
   Max DB Connections: 1000
   Connection Lifetime: 1s
   JWT Secret: ✓ Custom secret configured
Attempting to connect to database...
✓ Database connection established successfully!
✓ CPU monitoring started (logging to cpu_stats.log)
Server listening on http://127.0.0.1:3000
```

**Note:**
- Retry automatico connessione DB ogni 2 secondi se fallisce
- Task di monitoraggio CPU spawna in background (log ogni 120 secondi)
- CORS abilitato per tutte le origini (development)

### Avvio Client Tauri

**Development con hot-reload:**
```pwsh
cd client
npm run tauri dev
```

Questo comando:
1. Avvia Vite dev server (porta 1420 di default)
2. Compila backend Rust di Tauri
3. Lancia applicazione desktop
4. Hot-reload automatico su modifiche frontend

**Build production:**
```pwsh
cd client
npm run tauri build
```

Output: `src-tauri/target/release/bundle/` con installer nativi Windows

**Preview build frontend:**
```pwsh
npm run build
npm run preview
```

### Verifica Funzionamento

**Test endpoint health:**
```pwsh
curl http://localhost:3000/
```

**Test login:**
```pwsh
curl -X POST http://localhost:3000/auth/login `
  -H "Content-Type: application/json" `
  -d '{"username":"test_user","password":"Test123!"}'  
```

**Logs in tempo reale:**
```pwsh
$env:RUST_LOG="server=debug,tower_http=debug"
cargo run
```
---

## 8. Architettura del Sistema

### Diagramma generale

![Diagramma generale](diagrammi/diagramma_generale.png)

### Descrizione dei layer

L'architettura implementata segue un pattern a strati con separazione chiara delle responsabilità:

#### 1. Transport Layer
- **HTTP/HTTPS**: Axum router con endpoint REST per CRUD operations
- **WebSocket**: Endpoint `/ws` per comunicazione bidirezionale real-time
- **Middleware stack**: CORS (tower-http) → Authentication → Membership

#### 2. Application Layer (Handlers)
**File**: `server/src/main.rs` + `server/src/services/*.rs`

Handler Axum organizzati per dominio:
- `auth::login_user`, `auth::register_user`
- `user::search_user_with_username`, `user::get_my_user`, `user::get_user_by_id`, `user::delete_my_account`
- `chat::list_chats`, `chat::create_chat`, `chat::get_chat_messages`
- `membership::invite_to_chat`, `membership::respond_to_invitation`, `membership::list_chat_members`, `membership::leave_chat`, `membership::remove_member`, `membership::update_member_role`, `membership::transfer_ownership`, `membership::clean_chat`
- `ws::ws_handler` (upgrade WebSocket)

**Responsabilità**: 
- Validazione input (validator crate)
- Orchestrazione chiamate a services
- Gestione Extension<User> e Extension<UserChatMetadata>
- Costruzione risposte HTTP (StatusCode, Json)

#### 3. Service Layer
**File**: `server/src/services/`

**Componenti**:
- **auth.rs**: Generazione JWT (24h expiry), verifica password bcrypt
- **user.rs**: Ricerca utenti, gestione profilo
- **chat.rs**: Creazione chat GROUP/PRIVATE, recupero messaggi (paginazione 100 msg)
- **membership.rs**: Gestione membri, inviti, ruoli (Owner/Admin/Member)

**Responsabilità**:
- Logica di business e validazioni complesse
- Trasformazioni Entity ↔ DTO
- Orchestrazione repository + UserMap/ChatMap per notifiche real-time
- Generazione messaggi di sistema (join, leave, invitation)

#### 4. Repository Layer
**File**: `server/src/repositories/`

**Pattern**: Trait `Create<Entity, CreateDTO>`, `Read<Entity, Id>`, `Update`, `Delete`

**Implementazioni**:
- **user.rs**: `find_by_username`, `search_by_username` (LIKE query)
- **chat.rs**: `find_by_users` (chat private tra 2 utenti), `find_many_by_user_id`, `count_members`
- **message.rs**: `find_many_by_chat` (paginazione con `before_date`), `delete_before`, `count_unread`
- **invitation.rs**: `find_pending_by_user`, `get_enriched_invitation` (JOIN con users + chats), `find_existing_invite`
- **user_chat_metadata.rs**: `find_many_by_user_id`, `find_many_by_chat_id`, `update_messages_received_until`

**Responsabilità**:
- Query SQL parametrizzate (sqlx compile-time check)
- Mapping risultati a Entity
- Gestione transazioni (implicitamente tramite pool)

#### 5. Real-time Layer (WebSocket)
**File**: `server/src/ws/`

**Componenti**:
- **UserMap** (`usermap.rs`): DashMap<i32, UnboundedSender<InternalSignal>>
  - Traccia utenti online
  - Invia segnali: Shutdown, AddChat, RemoveChat, Error, Invitation
  
- **ChatMap** (`chatmap.rs`): DashMap<i32, broadcast::Sender<Arc<MessageDTO>>>
  - Canali broadcast per ogni chat (capacity 100)
  - Subscribe/unsubscribe dinamico
  - Rimozione automatica canali senza receiver
  
- **Connection Handler** (`connection.rs`):
  - `handle_socket`: Entry point, split WebSocket, spawn 2 task
  - `listen_ws`: Riceve messaggi client, rate limiting (10ms), timeout (300s)
  - `write_ws`: Batching messaggi (10 msg o 1 sec), gestione segnali interni

- **Event Handlers** (`event_handlers.rs`):
  - `process_message`: Validazione, membership check, broadcast + persist

**Costanti** (`mod.rs`):
```rust
BROADCAST_CHANNEL_CAPACITY: 100
BATCH_INTERVAL: 1000ms
BATCH_MAX_SIZE: 10
RATE_LIMITER_MILLIS: 10ms
TIMEOUT_DURATION_SECONDS: 300s
```

#### 6. Persistence Layer
**Database**: MySQL 8.0.43, Engine InnoDB, Charset utf8mb4_unicode_ci

**Tabelle**:
- `users` (user_id PK, username UNIQUE, password TEXT bcrypt)
- `chats` (chat_id PK, title, description, chat_type ENUM)
- `messages` (message_id PK, chat_id FK, sender_id FK, content, message_type ENUM, created_at)
- `invitations` (invite_id PK, target_chat_id FK, invited_id FK, invitee_id FK, state ENUM, created_at)
- `userchatmetadata` (PK composita: chat_id+user_id, user_role ENUM, member_since, messages_visible_from, messages_received_until)

**Indici ottimizzati**:
- `idx_Messages_chat_createdAt` (chat_id, created_at DESC) - Query messaggi recenti
- `idx_Messages_sender` (sender_id) - Query messaggi per utente
- `uq_Invitations_group_user_status` (target_chat_id, invited_id, state) - Prevenire inviti duplicati

#### 7. Monitoring Layer
**File**: `server/src/monitoring.rs`

**Componente**: `start_cpu_monitoring`
- Task tokio background spawned in `main.rs`
- Intervallo: 120 secondi (2 minuti)
- Raccoglie 120 campioni (1/sec) tramite `sysinfo::System`
- Calcola: avg CPU%, tempo CPU usato, memoria MB
- Output: `cpu_stats.log` (append con timestamp)

### Comunicazioni

**HTTP REST**:
- `/auth/*` - Login/register (no auth)
- `/users/*` - CRUD utenti (auth required)
- `/chats/*` - CRUD chat, messaggi (auth + membership required per alcune route)
- `/invitations/*` - Gestione inviti (auth required)

**WebSocket** (`/ws`):
1. Client → Upgrade HTTP con Bearer token
2. `authentication_middleware` → valida JWT → Extension<User>
3. `ws_handler` → Upgrade → `handle_socket(ws, state, user_id)`
4. Split socket → spawn `listen_ws` + `write_ws`
5. Client ↔ Server: MessageDTO (JSON)
6. Server → Client: Batch messages, segnali (AddChat, RemoveChat, Invitation, Error)

---

## 9. Architettura WebSocket

Questa sezione descrive in dettaglio l'implementazione del modulo WebSocket (`server/src/ws`).

### 9.0 Diagrammi architetturali

#### 9.0.1 Overview generale

Il seguente diagramma mostra una panoramica ad alto livello dell'architettura WebSocket:

![WebSocket Overview](diagrammi/ws_overview_1.png)

**Componenti principali**:
- **ws_handler**: Entry point per upgrade HTTP → WebSocket
- **handle_socket**: Orchestratore che gestisce split socket e spawn task
- **UserMap**: DashMap<user_id, UnboundedSender<InternalSignal>> - Utenti online
- **ChatMap**: DashMap<chat_id, broadcast::Sender<Arc<MessageDTO>>> - Canali broadcast per chat
- **listen_ws**: Task reader (client → server)
- **write_ws**: Task writer (server → client)

#### 9.0.2 Connection Lifecycle

Diagramma dettagliato del ciclo di vita di una connessione WebSocket (setup → running → cleanup):

![WebSocket Lifecycle](diagrammi/ws_connection_lifecycle_2.png)

**Fasi**:
1. **Setup**: Middleware auth → Upgrade → split(ws) → unbounded_channel<InternalSignal>
2. **Registration**: UserMap.register_online(user_id, internal_tx)
3. **Running**: Spawn listen_ws + write_ws concorrenti
4. **Cleanup**: Timeout/Close → InternalSignal::Shutdown → UserMap.remove_from_online

**Parametri implementati**:
- **Rate limiting**: 10ms tra letture client (~100 msg/sec per connessione)
- **Timeout inattività**: 300 secondi (5 minuti)
- **Acquire timeout DB**: 2 secondi (per caricare chat utente in write_ws)
- **Cleanup automatico**: Rimozione da UserMap + terminazione task

#### 9.0.3 Message Flow

Diagramma del flusso completo di un messaggio: client → validazione → persistenza → broadcast:

![WebSocket Message Flow](diagrammi/ws_message_flow_3.png)

**Flusso implementato**:
1. **Client → Server**: 
   - `listen_ws` riceve Message::Text
   - Deserializza JSON → MessageDTO
   - Conversione → CreateMessageDTO (validazione content 1-5000 char)
   - Verifica sender_id == user_id autenticato (anti-spoofing)
   - Blocco MessageType::SystemMessage da client
   
2. **Validazione Membership**:
   - Query `state.meta.read((user_id, chat_id))`
   - Se None → InternalSignal::Error("Membership not found")
   
3. **Broadcast**:
   - `state.chats_online.send(&chat_id, Arc::from(msg))`
   - Broadcast::send a tutti i receiver attivi della chat
   - Se 0 receiver → canale rimosso da ChatMap
   
4. **Persistenza**:
   - `state.msg.create(&input_message)` (SEMPRE, anche se 0 utenti online)
   - Messaggio disponibile per fetch successivo
   
5. **Batching**:
   - write_ws accumula messaggi in Vec<Arc<MessageDTO>>
   - Invio quando: batch.len() >= 10 OPPURE interval.tick() (1 sec)
   - JSON: `[{message_id:1,...}, {message_id:2,...}]`

**Gestione utenti offline**:
- Il messaggio è persistito anche se nessun receiver online
- Al prossimo login, l'utente recupera messaggi con GET `/chats/{chat_id}/messages`

#### 9.0.4 Internal Signals

Diagramma dei segnali interni (Invitation, AddChat, RemoveChat, Error, Shutdown):

![WebSocket Internal Signals](diagrammi/ws_internal_signals_4.png)

**Segnali implementati** (enum `InternalSignal` in `usermap.rs`):

```rust
pub enum InternalSignal {
    Shutdown,                                    // Chiudi connessione
    AddChat(i32),                               // Sottoscrivi a nuova chat
    RemoveChat(i32),                            // Rimuovi sottoscrizione
    Error(&'static str),                        // Notifica errore
    Invitation(EnrichedInvitationDTO),          // Notifica invito
}
```

**Meccanismo**:
1. Services/Repositories chiamano `state.users_online.send_server_message_if_online(&user_id, signal)`
2. UserMap fa lookup in DashMap e invia sul `UnboundedSender<InternalSignal>`
3. write_ws riceve dal `internal_rx` channel
4. Serializza segnale e invia JSON al client:
   - `{"AddChat": 123}` → Client sottoscrive chat_id 123
   - `{"RemoveChat": 456}` → Client rimuove chat_id 456
   - `{"Invitation": {...}}` → Client mostra notifica invito
   - `{"Error": "message"}` → Client mostra errore
   - `Shutdown` → write_ws invia batch finale e chiude

**Esempi d'uso**:
- **Accept invite**: `membership::respond_to_invitation` → `users_online.send_server_message_if_online(&invited_id, AddChat(chat_id))`
- **Remove member**: `membership::remove_member` → `send_server_message_if_online(&removed_user_id, RemoveChat(chat_id))`
- **Validation error**: `process_message` → `send_server_message_if_online(&user_id, Error("Invalid message"))`


### 9.1 Analisi approfondita `connection.rs`

**Ruolo**: Orchestrare la vita di una singola connessione WebSocket per utente autenticato.

#### Funzione `handle_socket`

```rust
pub async fn handle_socket(ws: WebSocket, state: Arc<AppState>, user_id: i32)
```

**Flusso completo**:
1. `ws.split()` → `(ws_tx: SplitSink, ws_rx: SplitStream)`
2. `unbounded_channel::<InternalSignal>()` → `(int_tx, int_rx)`
3. `state.users_online.register_online(user_id, int_tx.clone())` → Registra utente
4. Spawn 2 task concorrenti:
   - `tokio::spawn(listen_ws(user_id, ws_rx, int_tx, state.clone()))`
   - `tokio::spawn(write_ws(user_id, ws_tx, int_rx, state))`

**Logging**: `info!("WebSocket connection established")`, `info!("User registered as online")`

#### Funzione `write_ws`

**Signature**:
```rust
async fn write_ws(
    user_id: i32,
    mut websocket_tx: SplitSink<WebSocket, Message>,
    mut internal_rx: UnboundedReceiver<InternalSignal>,
    state: Arc<AppState>,
)
```

**Flusso dettagliato**:
1. **Caricamento chat utente**:
   ```rust
   let chat_vec: Vec<i32> = state.meta.find_many_by_user_id(&user_id).await
       .map(|chats| chats.iter().map(|m| m.chat_id).collect())
   ```
   - Se errore DB → termina task (`return`)

2. **Sottoscrizione canali broadcast**:
   ```rust
   let mut stream_map = StreamMap::new();
   state.chats_online.subscribe_multiple(chat_vec)
       .into_iter()
       .zip(chat_vec.into_iter())
       .for_each(|(rx, chat_id)| {
           stream_map.insert(chat_id, BroadcastStream::new(rx));
       });
   ```

3. **Loop principale**:
   ```rust
   let mut batch: Vec<Arc<MessageDTO>> = Vec::new();
   let mut interval = tokio::time::interval(Duration::from_millis(BATCH_INTERVAL));
   
   loop {
       tokio::select! {
           Some((_, result)) = stream_map.next() => {
               // Messaggio da broadcast
               if let Ok(msg) = result {
                   batch.push(msg);
                   if batch.len() >= BATCH_MAX_SIZE {
                       send_batch(&mut websocket_tx, &batch).await;
                       batch.clear();
                   }
               }
           }
           
           _ = interval.tick() => {
               // Tick periodico
               if !batch.is_empty() {
                   send_batch(&mut websocket_tx, &batch).await;
                   batch.clear();
               }
           }
           
           Some(signal) = internal_rx.recv() => {
               // Segnali interni
               match signal {
                   InternalSignal::Shutdown => break,
                   InternalSignal::AddChat(chat_id) => {
                       let rx = state.chats_online.subscribe(&chat_id);
                       stream_map.insert(chat_id, BroadcastStream::new(rx));
                       // Invia JSON: {"AddChat": chat_id}
                   }
                   InternalSignal::RemoveChat(chat_id) => {
                       stream_map.remove(&chat_id);
                       // Invia JSON: {"RemoveChat": chat_id}
                   }
                   InternalSignal::Error(err) => {
                       // Invia JSON: {"Error": err}
                   }
                   InternalSignal::Invitation(inv) => {
                       // Invia JSON: {"Invitation": inv}
                   }
               }
           }
       }
   }
   ```

4. **Cleanup**:
   ```rust
   // Invia batch finale
   if !batch.is_empty() {
       send_batch(&mut websocket_tx, &batch).await;
   }
   // Rimuovi utente da UserMap
   state.users_online.remove_from_online(&user_id);
   ```

#### Funzione `listen_ws`

**Signature**:
```rust
async fn listen_ws(
    user_id: i32,
    mut websocket_rx: SplitStream<WebSocket>,
    internal_tx: UnboundedSender<InternalSignal>,
    state: Arc<AppState>,
)
```

**Flusso dettagliato**:
1. **Setup rate limiter e timeout**:
   ```rust
   let mut rate_limiter = interval(Duration::from_millis(RATE_LIMITER_MILLIS));
   let timeout_duration = Duration::from_secs(TIMEOUT_DURATION_SECONDS);
   ```

2. **Loop principale**:
   ```rust
   loop {
       match timeout(timeout_duration, StreamExt::next(&mut websocket_rx)).await {
           Ok(Some(Ok(Message::Text(text)))) => {
               rate_limiter.tick().await; // Rate limiting
               
               // Deserializza MessageDTO
               match serde_json::from_str::<MessageDTO>(&text) {
                   Ok(msg) => {
                       // Process message
                       process_message(user_id, msg, state.clone()).await;
                   }
                   Err(_) => {
                       // Invia errore al client
                       state.users_online.send_server_message_if_online(
                           &user_id, 
                           InternalSignal::Error("Malformed message")
                       );
                   }
               }
           }
           
           Ok(Some(Ok(Message::Close(_)))) => {
               // Client disconnessione
               break;
           }
           
           Err(_) => {
               // Timeout inattività
               warn!(timeout_secs = TIMEOUT_DURATION_SECONDS, "Connection timeout");
               break;
           }
           
           _ => {}
       }
   }
   ```

3. **Cleanup**:
   ```rust
   internal_tx.send(InternalSignal::Shutdown).ok();
   info!("WebSocket connection closed");
   ```

### 9.2 Analisi approfondita `chatmap.rs`

**Ruolo**: Mantenere mappa thread-safe di canali broadcast per ogni chat.

**Struct**:
```rust
pub struct ChatMap {
    chats: DashMap<i32, broadcast::Sender<Arc<MessageDTO>>>,
}
```

#### Metodi implementati

**`subscribe`**:
```rust
pub fn subscribe(&self, chat_id: &i32) -> broadcast::Receiver<Arc<MessageDTO>>
```
- Se canale esiste → `tx.subscribe()` (nuovo receiver)
- Se non esiste → `broadcast::channel(BROADCAST_CHANNEL_CAPACITY)` → insert → ritorna rx
- Capacity: 100 messaggi (costante)

**`subscribe_multiple`**:
```rust
pub fn subscribe_multiple(&self, chat_ids: Vec<i32>) -> Vec<broadcast::Receiver<Arc<MessageDTO>>>
```
- Chiama `subscribe` per ogni chat_id
- Ritorna Vec di receiver (stesso ordine)

**`send`**:
```rust
pub fn send(&self, chat_id: &i32, msg: Arc<MessageDTO>) -> Result<usize, SendError<Arc<MessageDTO>>>
```
- Lookup in DashMap: `self.chats.get(chat_id)`
- Se esiste → `tx.send(msg)` → ritorna numero receiver che hanno ricevuto
- Se `send()` fallisce (0 receiver) → `self.chats.remove(chat_id)` (cleanup automatico)
- Se canale non esiste → ritorna Err

**`is_active`**:
```rust
pub fn is_active(&self, chat_id: &i32) -> bool
```
- Controlla se `self.chats.contains_key(chat_id)`

**Trade-offs**:
- `Arc<MessageDTO>` evita clone del messaggio per ogni receiver (efficienza memoria)
- Canali creati on-demand (lazy initialization)
- Rimozione automatica canali senza receiver (evita memory leak)

### 9.3 Task di lettura/scrittura

**Separazione responsabilità**:
- **listen_ws**: Input handling (client → server)
  - Rate limiting anti-flood
  - Timeout inattività
  - Deserializzazione e validazione
  - Invocazione `process_message`
  
- **write_ws**: Output handling (server → client)
  - Batching efficiente
  - Gestione segnali interni
  - Sottoscrizione dinamica chat

**Concorrenza**:
- Entrambi i task girano concorrentemente grazie a tokio::spawn
- Comunicano tramite `internal_channel` (unbounded, async)
- Shutdown coordinato: listen_ws → send(Shutdown) → write_ws riceve e termina

### 9.4 Segnali interni

**Utilizzo nei services**:

**Esempio: Invite to chat** (`membership.rs::invite_to_chat`):
```rust
let enriched = state.invitation.get_enriched_invitation(&created.invite_id).await?;
state.users_online.send_server_message_if_online(
    &invited_id,
    InternalSignal::Invitation(enriched)
);
```

**Esempio: Accept invitation** (`membership.rs::respond_to_invitation` con action=accept):
```rust
// ... crea metadata ...
state.users_online.send_server_message_if_online(
    &user_id,
    InternalSignal::AddChat(invitation.target_chat_id)
);
```

**Esempio: Remove member** (`membership.rs::remove_member`):
```rust
// ... elimina metadata ...
state.users_online.send_server_message_if_online(
    &user_id_to_remove,
    InternalSignal::RemoveChat(chat_id)
);
```

### 9.5 Gestione utenti

**UserMap** (`usermap.rs`):
```rust
pub struct UserMap {
    users: DashMap<i32, UnboundedSender<InternalSignal>>,
}
```

**Metodi**:
- `register_online(user_id, tx)` → Insert in DashMap
- `remove_from_online(&user_id)` → Remove from DashMap
- `send_server_message_if_online(&user_id, signal)` → Lookup + tx.send(signal)
- `is_online(&user_id)` → contains_key
- `online_count()` → DashMap.len()

**Lifecycle**:
1. `handle_socket` → `register_online` (all'inizio)
2. Durante connessione → `send_server_message_if_online` da services
3. `write_ws` termina → `remove_from_online` (cleanup)

### 9.6 Messaggi e formati

**DTO principale**: `MessageDTO` (server/src/dtos/message.rs)
```rust
pub struct MessageDTO {
    pub message_id: Option<i32>,
    pub chat_id: Option<i32>,
    pub sender_id: Option<i32>,
    pub content: String,
    pub message_type: MessageType, // UserMessage | SystemMessage
    pub created_at: DateTime<Utc>,
}
```

**Client → Server (invio messaggio)**:
```json
{
  "chat_id": 10,
  "sender_id": 42,
  "content": "Ciao mondo!",
  "message_type": "UserMessage",
  "created_at": "2025-11-25T14:30:00Z"
}
```

**Server → Client (batch)**:
```json
[
  {
    "message_id": 123,
    "chat_id": 10,
    "sender_id": 2,
    "content": "Messaggio 1",
    "message_type": "UserMessage",
    "created_at": "2025-11-25T14:29:00Z"
  },
  {
    "message_id": 124,
    "chat_id": 10,
    "sender_id": 3,
    "content": "Messaggio 2",
    "message_type": "SystemMessage",
    "created_at": "2025-11-25T14:29:30Z"
  }
]
```

**Server → Client (segnali)**:
```json
// AddChat
{"AddChat": 15}

// RemoveChat
{"RemoveChat": 15}

// Error
{"Error": "Invalid message format"}

// Invitation
{
  "Invitation": {
    "invite_id": 10,
    "state": "PENDING",
    "created_at": "2025-11-25T14:30:00Z",
    "inviter": {
      "user_id": 5,
      "username": "mario_rossi"
    },
    "chat": {
      "chat_id": 15,
      "title": "Team Project",
      "description": "Chat di gruppo",
      "chat_type": "GROUP"
    }
  }
}
```

### 9.7 Esempi dettagliati

#### Esempio 1: Invio messaggio da client

**Flusso completo**:
1. **Client** (Tauri): `invoke('send_websocket_message', {message: {...}})`
2. **Tauri backend**: Serializza JSON → WebSocket.send(text)
3. **Server listen_ws**: Riceve `Message::Text(text)`
   - `rate_limiter.tick().await` (10ms delay)
   - `serde_json::from_str::<MessageDTO>(&text)` → `msg_dto`
4. **process_message** (`event_handlers.rs`):
   ```rust
   // Conversione a CreateMessageDTO (validazione)
   let input_message = CreateMessageDTO::try_from(msg_dto)?;
   input_message.validate()?; // content 1-5000 char
   
   // Anti-spoofing
   if input_message.sender_id != user_id {
       return Error("Sender ID mismatch");
   }
   
   // Blocco SystemMessage da client
   if input_message.message_type == MessageType::SystemMessage {
       return Error("Cannot send system messages");
   }
   
   // Verifica membership
   let metadata = state.meta.read((user_id, input_message.chat_id)).await?;
   
   // Broadcast
   let arc_msg = Arc::from(msg_dto);
   state.chats_online.send(&chat_id, arc_msg.clone());
   
   // Persist
   state.msg.create(&input_message).await?;
   ```
5. **Server write_ws** (altri client della stessa chat):
   - Ricevono da `BroadcastStream`
   - Accumulano in `batch`
   - Inviano quando batch.len() >= 10 o interval.tick()

#### Esempio 2: Server notifica invito

**Flusso**:
1. **Service**: `membership::invite_to_chat`
   ```rust
   let invitation = state.invitation.create(&CreateInvitationDTO {
       target_chat_id,
       invited_id,
       invitee_id: user_id, // Inviter
   }).await?;
   
   let enriched = state.invitation.get_enriched_invitation(&invitation.invite_id).await?;
   
   state.users_online.send_server_message_if_online(
       &invited_id,
       InternalSignal::Invitation(enriched)
   );
   ```
2. **UserMap**:
   ```rust
   if let Some(tx) = self.users.get(&invited_id) {
       tx.send(InternalSignal::Invitation(enriched)).ok();
   }
   ```
3. **write_ws** (connessione invited_id):
   ```rust
   Some(InternalSignal::Invitation(inv)) = internal_rx.recv() => {
       let json = serde_json::to_string(&json!({"Invitation": inv}))?;
       websocket_tx.send(Message::Text(json)).await;
   }
   ```
4. **Client** (Tauri):
   - Listener `ws-message` riceve JSON
   - Parse `data.Invitation` → callback `onInvitation`
   - Mostra notifica: `notifyNewInvitation(invitation)`

---

## 10. Struttura del Progetto

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

## 11. API Documentation

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

## 12. WebSocket Protocol Documentation

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

## 13. Database Schema

### Diagramma ER

![Database Schema](diagrammi/ruggineDB_ER.jpg)

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

## 14. Test

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

## 15. Logging

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

## 16. Deployment & Context Diagram

### Deployment Diagram

![Deployment Diagram](diagrammi/deployment_diagram.png)

### Context Diagram

![Context Diagram](diagrammi/context_diagram.png)

### Descrizione componenti

**Implementazione attuale (deployment locale)**:

- **Client Desktop**: Applicazione Tauri (React + TypeScript bundled in eseguibile nativo Windows)
  - Comunicazione diretta con server via HTTP/HTTPS e WebSocket
  - Notifiche native OS tramite `@tauri-apps/plugin-notification`
  
- **Server Application**: Binario Rust compilato (`server.exe`)
  - Ascolto su `127.0.0.1:3000` (configurabile via `SERVER_HOST` e `SERVER_PORT`)
  - Gestisce endpoint REST e upgrade WebSocket `/ws`
  - CORS abilitato per development
  
- **Database**: MySQL 8.0.43+
  - Connessione via pool sqlx (max 1000 connessioni, lifetime 1s)
  - Schema: `rugginedb` con 5 tabelle (users, chats, messages, invitations, userchatmetadata)
  - Utente dedicato: `ruggine` con privilegi limitati

### Setup deployment

**Build e avvio locale**:

1. **Server**:
   ```pwsh
   cd server
   cargo build --release
   .\target\release\server.exe
   ```

2. **Client Tauri**:
   ```pwsh
   cd client
   npm run tauri build
   # Installer in: src-tauri/target/release/bundle/
   ```

3. **Database**:
   ```pwsh
   mysql -u root -p < server/migrations/1_create_database.sql
   ```

---

## 17. Documentazione Client

### Architettura front-end

- **Framework**: React 19.1.0 + TypeScript 5.8.3 + Vite 7.0.4
- **UI Library**: Bootstrap 5.3.8 + React Bootstrap 2.10.10 + Bootstrap Icons 1.13.1
- **Desktop**: Tauri 2.x (wrapper nativo, WebSocket gestito tramite `@tauri-apps/api`)
- **State Management**: Context API
  - `AuthContext`: gestione autenticazione (login, register, logout, user state)
  - `WebSocketContext`: connessione WS condivisa, gestione messaggi real-time
- **Servizi**: 
  - `services/api.ts`: chiamate HTTP REST (fetch con headers JWT)
  - `services/tauri.ts`: file vuoto (non utilizzato)

### Routing (React Router DOM 7.9.5)

**Struttura route** (`App.tsx`):
- `/login` - Pagina pubblica di autenticazione
- `/home` - Pagina protetta (richiede `ProtectedRoute`)
- `/` - Redirect a `/home`
- `*` (404) - Redirect a `/home`

**Componente `ProtectedRoute`**: controlla `isAuthenticated` da `AuthContext`, se false redirige a `/login`.

### Pagine implementate

**`pages/Login/Login.tsx`**:
- Form con username/password
- Bottoni: "Accedi" (login) e "Registrati" (register)
- Chiamate: `POST /auth/login` o `POST /auth/register`
- Al successo: salva JWT in localStorage, setta `user` in AuthContext, redirect a `/home`

**`pages/Home/Home.tsx`**:
- Layout a 2 colonne: `Sidebar` (sinistra) + area chat (destra)
- Area chat contiene: `ChatHeader`, `ChatArea`, `ChatInput`, `ChatInfo`
- Gestisce selezione chat corrente e stato UI

### Componenti principali

**`Sidebar/`**:
- Lista chat dell'utente (chiama `GET /chats`)
- Mostra `SideChatCard` per ogni chat
- Pulsanti: "Nuova Chat", "Inviti Pendenti", "Profilo"

**`SideChatCard/`**:
- Card singola chat con titolo, ultimo messaggio, badge unread
- Click → seleziona chat in `Home`

**`ChatHeader/`**:
- Header chat con titolo, descrizione, tipo (GROUP/PRIVATE)
- Pulsanti: info chat, gestione membri (se admin/owner)

**`ChatArea/`**:
- Scroll area con lista messaggi
- Carica messaggi: `GET /chats/{chat_id}/messages`
- Subscribe a `WebSocketContext.subscribeToChat()` per nuovi messaggi real-time
- Renderizza `ChatMessage` per ogni messaggio

**`ChatMessage/`**:
- Singolo messaggio con avatar, username, contenuto, timestamp
- Stile diverso per messaggi propri vs altrui
- Supporta `MessageType.USERMESSAGE` e `MessageType.SYSTEMMESSAGE`

**`ChatInput/`**:
- Textarea per scrivere messaggi
- Bottone invio → `webSocketContext.sendMessage(messageDTO)`
- Crea `MessageDTO` con `chat_id`, `sender_id`, `content`, `message_type`, `created_at`

**`ChatInfo/`**:
- Sidebar destra (opzionale) con dettagli chat
- Lista membri con ruoli (OWNER/ADMIN/MEMBER)
- Azioni: invita utente, rimuovi membro, cambia ruolo, trasferisci ownership

**`ProfileModal/`**:
- Modal con info utente corrente
- Bottone logout

**`ProtectedRoute.tsx`**:
- HOC che wrappa route protette
- Controlla `isAuthenticated`, altrimenti `<Navigate to="/login" />`

### Context e Hooks

**`AuthContext`** (`context/AuthContext.tsx`):
```tsx
interface AuthContextType {
  user: UserDTO | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (username: string, password: string) => Promise<void>;
  register: (username: string, password: string) => Promise<void>;
  logout: () => void;
}
```
- Gestisce token JWT in localStorage (`token` key)
- `useEffect` iniziale: carica user da `GET /users/me` se token presente

**`WebSocketContext`** (`context/WebSocketContext.tsx`):
```tsx
interface WebSocketContextType {
  isConnected: boolean;
  sendMessage: (message: MessageDTO) => void;
  subscribeToChat: (chatId: number, callback: (message: MessageDTO) => void) => () => void;
  onError: (callback: (error: string) => void) => () => void;
  onChatAdded: (callback: (chatId: number) => void) => () => void;
  onChatRemoved: (callback: (chatId: number) => void) => () => void;
  onInvitation: (callback: (invitation: EnrichedInvitationDTO) => void) => () => void;
}
```
- **Implementazione Tauri**: usa `@tauri-apps/api` (`invoke`, `listen`)
- Connessione WS gestita da backend Rust di Tauri (src-tauri)
- Eventi server: batch messaggi, `AddChat`, `RemoveChat`, `Invitation`, `Error`
- Notifiche desktop: `useNotifications` hook (`@tauri-apps/plugin-notification`)

**`useNotifications`** (`hooks/useNotifications.ts`):
- Hook custom per notifiche desktop native
- Richiede permesso utente, mostra notifiche per nuovi messaggi/inviti

### Interazioni API e WebSocket

**HTTP REST** (tramite `services/api.ts`):
- Autenticazione: `POST /auth/login`, `POST /auth/register`
- Utenti: `GET /users`, `GET /users/me`, `GET /users/{user_id}`
- Chat: `GET /chats`, `POST /chats`, `GET /chats/{chat_id}/messages`, `GET /chats/{chat_id}/members`
- Membership: `POST /chats/{chat_id}/invite/{user_id}`, `PATCH /chats/{chat_id}/members/{user_id}/role`, etc.
- Inviti: `GET /invitations/pending`, `POST /invitations/{invite_id}/{action}`

**WebSocket** (tramite Tauri backend):
- Client → Server: `MessageDTO` (invio messaggi)
- Server → Client:
  - Batch messaggi (array `MessageDTO[]`)
  - Segnali: `{"AddChat": chatId}`, `{"RemoveChat": chatId}`, `{"Invitation": enrichedDTO}`, `{"Error": msg}`
- Token JWT passato in header `Authorization: Bearer <token>` durante upgrade

---

## 18. Dimensione del Compilato

### Comandi per misurare build

**Server Rust**:

**Build release** (ottimizzato):
```pwsh
cd server
cargo build --release
Get-Item target\release\server.exe | Select-Object Name,@{Name='SizeMB';Expression={[math]::Round($_.Length/1MB,2)}}
```
Dimensione eseguibile: 2.86 MB

**Client Tauri (installer desktop completo)**:
```pwsh
cd client
npm run tauri build
# Windows installer in: src-tauri\target\release\bundle\nsis\
Get-Item src-tauri\target\release\bundle\nsis\*.exe | Select-Object Name,@{Name='SizeMB';Expression={[math]::Round($_.Length/1MB,2)}}
```
Dimensione eseguibile: 11.8 MB

### Ottimizzazioni implementate

**Server** (`server/Cargo.toml` - `[profile.release]`):
```toml
opt-level = "z"        # Ottimizzazione massima per dimensione (z = size, 3 = speed)
lto = true             # Link-Time Optimization (già abilitato)
codegen-units = 1      # Single codegen unit per migliori ottimizzazioni
panic = "abort"        # Panic abort invece di unwind (riduce size)
strip = true           # Rimuove simboli debug automaticamente
```
---

## 19. Sicurezza

### Autenticazione JWT

**Implementazione** (`server/src/core/auth.rs`):
- Algoritmo: **HS256** (HMAC con SHA-256)
- Durata token: **24 ore** (configurato in `encode_jwt`)
- Claims: `username`, `id` (user_id), `iat` (issued at), `exp` (expiration)
- Secret: configurabile via `JWT_SECRET` env var
  - Default insicuro: `"supersecretkey"` (warning al boot se non configurato)
  - **Produzione**: usare secret robusto (min 32 caratteri alfanumerici casuali)

**Validazione**:
- Middleware `authentication_middleware` verifica JWT in header `Authorization: Bearer <token>`
- Controlla scadenza automaticamente via `jsonwebtoken` crate
- Estrae user e inserisce `Extension<User>` per i handler

### Hash Password

**Implementazione** (`server/src/entities/user.rs`):
```rust
use bcrypt::{DEFAULT_COST, hash, verify};

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    let hash = hash(password, DEFAULT_COST)?;
    Ok(hash)
}
```

- Algoritmo: **bcrypt** (versione 0.17.1)
- Cost factor: `DEFAULT_COST` = **12** (2^12 = 4096 iterazioni)
- Salt: generato automaticamente da bcrypt (random per ogni password)
- Storage: hash come TEXT in tabella `users.password`

### Protezione SQL Injection

**Implementazione** (`server/src/repositories/*.rs`):
- Tutte le query usano **parametrizzazione sqlx**: `sqlx::query!` e `sqlx::query_as!`
- Esempio sicuro:
  ```rust
  sqlx::query_as!(User, "SELECT * FROM users WHERE username = ?", username)
  ```
- **Zero concatenazione stringhe** nelle query SQL
- Compile-time check delle query (feature `macros` di sqlx)


### Permessi Database

**Configurazione** (`server/migrations/1_create_database.sql`):

Lo script di migrazione contiene (commentati) i comandi per creare utente con privilegi limitati:

```sql
-- CREATE USER IF NOT EXISTS 'ruggine'@'localhost' IDENTIFIED BY 'ferro';
-- GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, INDEX, ALTER 
--   ON rugginedb.* TO 'ruggine'@'localhost';
```

### Rate Limiting WebSocket

**Implementazione** (`server/src/ws/connection.rs`):

```rust
const RATE_LIMITER_MILLIS: u64 = 10; // ~100 msg/sec per connessione
const TIMEOUT_DURATION_SECONDS: u64 = 300; // 5 minuti inattività

let mut rate_limiter = interval(Duration::from_millis(RATE_LIMITER_MILLIS));

// Loop di lettura messaggi
rate_limiter.tick().await; // Attende 10ms tra ogni messaggio
```

---

## 20. Performance

### Ottimizzazioni WebSocket

**Batching messaggi** (`server/src/ws/connection.rs`):

```rust
const BATCH_INTERVAL: u64 = 1000;      // 1 secondo
const BATCH_MAX_SIZE: usize = 10;      // 10 messaggi

let mut batch: Vec<Arc<MessageDTO>> = Vec::new();
let mut interval = tokio::time::interval(Duration::from_millis(BATCH_INTERVAL));

// Accumula messaggi e invia quando:
// - batch.len() >= 10 messaggi OPPURE
// - interval tick (1 secondo)
```

**Zero-copy con Arc** (`server/src/ws/chatmap.rs`):

```rust
use std::sync::Arc;

pub struct ChatMap {
    channels: DashMap<i32, Sender<Arc<MessageDTO>>>,
}

pub fn send(&self, chat_id: &i32, msg: Arc<MessageDTO>) -> Result<usize, SendError<Arc<MessageDTO>>>
```

### Concorrenza Lock-Free

**DashMap** (versione 6.1.0):

```rust
use dashmap::DashMap;

pub struct UserMap {
    users_online: DashMap<i32, UnboundedSender<InternalSignal>>,
}

pub struct ChatMap {
    channels: DashMap<i32, Sender<Arc<MessageDTO>>>,
}
```

**tokio::sync::broadcast** (multi-producer multi-consumer):

```rust
use tokio::sync::broadcast;

let (tx, _) = broadcast::channel::<Arc<MessageDTO>>(100); // Capacity 100
let rx1 = tx.subscribe();
let rx2 = tx.subscribe();

tx.send(Arc::new(message))?; // Tutti i receiver ricevono
```

### Async Runtime

**Tokio** (versione 1.47.1):

```rust
tokio = { version = "1.47.1", features = [
    "rt-multi-thread",    // Runtime multi-thread
    "macros",             // #[tokio::main]
    "net",                // TcpListener async
    "io-util",            // AsyncRead/AsyncWrite
    "time",               // interval, timeout
    "sync",               // broadcast, mpsc, oneshot
] }
```

### Database Connection Pool

**sqlx** (versione 0.8.6):

```rust
sqlx = { version = "0.8.6", features = [
    "mysql",                      // Driver MySQL
    "runtime-tokio-native-tls",   // Async runtime + TLS
    "macros",                     // query! compile-time check
] }
```

**Configurazione pool** (`server/src/core/config.rs`):

```rust
MAX_DB_CONNECTIONS = 1000        // Pool max size
DB_CONNECTION_LIFETIME_SECS = 1  // Lifetime connessione
ACQUIRE_TIMEOUT = 2 seconds      // Timeout acquire
TEST_BEFORE_ACQUIRE = true       // Verifica connessione prima di restituirla
```

### Logging e Tracing

**Stack** (`server/Cargo.toml`):
```rust
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Configurazione** (`server/src/main.rs`):
```rust
use tracing_subscriber::{fmt, EnvFilter};

tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();
```

**Livelli configurabili tramite variabile d'ambiente**:
```pwsh
# Development
$env:RUST_LOG = "server=debug,tower_http=debug"
cargo run

# Production
$env:RUST_LOG = "server=info,tower_http=warn"
cargo run --release
```

### Ottimizzazioni Compilatore

**Profile release** (`server/Cargo.toml`):

```toml
[profile.release]
opt-level = "z"        # Ottimizza per size (z) invece di speed (3)
lto = true             # Link-Time Optimization (inlining cross-crate)
codegen-units = 1      # Singolo codegen unit (più ottimizzazioni, compile più lento)
panic = "abort"        # Panic abort invece di unwind (riduce size)
strip = true           # Rimuove simboli debug
```

```

### Metriche Performance Attese

