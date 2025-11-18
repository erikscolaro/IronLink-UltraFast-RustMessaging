# Ruggine Server

Server backend per l'applicazione di chat **Ruggine**, sviluppato in Rust con architettura REST API + WebSocket per comunicazioni real-time.

## Indice

- [Panoramica](#panoramica)
- [Architettura](#architettura)
- [Tecnologie](#tecnologie)
- [Installazione](#installazione)
- [Configurazione](#configurazione)
- [Avvio](#avvio)
- [API Documentation](#api-documentation)
- [WebSocket Protocol](#websocket-protocol)
- [Database Schema](#database-schema)
- [Testing](#testing)
- [Struttura del Progetto](#struttura-del-progetto)

---

## Panoramica

Il server Ruggine è un sistema di messaggistica completo che supporta:

- **Autenticazione JWT** - Sistema di autenticazione sicuro con token Bearer
- **Chat Unificate** - Supporto per chat private e di gruppo con un unico modello
- **WebSocket Real-time** - Messaggistica istantanea con gestione connessioni persistenti
- **Sistema di Inviti** - Gestione inviti con stati (pending, accepted, rejected)
- **Gestione Ruoli** - Sistema di permessi (Owner, Admin, Standard)
- **Database MySQL** - Persistenza dati con SQLx e query compile-time checked
- **Connection Pooling** - Gestione efficiente delle connessioni al database
- **Middleware Avanzati** - Autenticazione e verifica membership

## Architettura

Il server adotta un'architettura a layer ben definita:

```
┌─────────────────────────────────────────┐
│         HTTP/WebSocket Layer            │
│    (Axum Router + WebSocket Handler)    │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│         Services Layer                  │
│  (Business Logic & Orchestration)       │
│  - auth, user, chat, membership         │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│        Repository Layer                 │
│    (Data Access & Persistence)          │
│  - CRUD traits implementation           │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│           Database Layer                │
│        (MySQL + SQLx Pool)              │
└─────────────────────────────────────────┘
```

### WebSocket Architecture

```
┌──────────────┐         ┌──────────────┐
│   Client A   │◄───────►│  UserMap     │
└──────────────┘         │  (DashMap)   │
                         └──────┬───────┘
┌──────────────┐                │
│   Client B   │◄───────────────┤
└──────────────┘                │
                         ┌──────▼───────┐
┌──────────────┐         │   ChatMap    │
│   Client C   │◄───────►│  (DashMap)   │
└──────────────┘         └──────────────┘
```

- **UserMap**: Mappa `user_id → Sender` per messaggi diretti agli utenti
- **ChatMap**: Mappa `chat_id → HashSet<user_id>` per broadcast ai membri di una chat

## Tecnologie

### Core Dependencies

| Crate | Versione | Utilizzo |
|-------|----------|----------|
| **axum** | 0.8.4 | Framework web asincrono con supporto WebSocket |
| **tokio** | 1.47.1 | Runtime asincrono |
| **sqlx** | 0.8.6 | Database driver MySQL con compile-time checks |
| **serde** | 1.0.226 | Serializzazione/deserializzazione JSON |
| **jsonwebtoken** | 9.3.1 | Gestione JWT per autenticazione |
| **bcrypt** | 0.17.1 | Hashing password |
| **dashmap** | 6.1.0 | HashMap thread-safe per WebSocket maps |
| **validator** | 0.18 | Validazione input con derive macro |
| **tracing** | 0.1 | Logging strutturato |
| **chrono** | 0.4.42 | Gestione date e timestamp |

### Dev Dependencies

- **axum-test** (18.1.0) - Testing framework per API REST
- **tokio-tungstenite** (0.24.0) - Testing WebSocket connections

## Installazione

### Prerequisiti

- **Rust** 1.70+ (edition 2024)
- **MySQL** 8.0+
- **Git**

### Setup

1. **Clone del repository**
   ```bash
   git clone https://github.com/PdS2425-C2/G43.git
   cd G43/server
   ```

2. **Installazione dipendenze**
   ```bash
   cargo build
   ```

3. **Setup database**
   ```bash
   # Avviare MySQL
   mysql -u root -p
   
   # Eseguire lo script di migrazione
   source migrations/1_create_database.sql
   ```

## Configurazione

Creare un file `.env` nella root del progetto server:

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

## Avvio

### Modalità Development

```bash
cargo run
```

### Modalità Release (ottimizzato)

```bash
cargo build --release
./target/release/server
```

### Con Hot Reload (cargo-watch)

```bash
cargo install cargo-watch
cargo watch -x run
```

### Output di Avvio

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

## API Documentation

### Base URL
```
http://127.0.0.1:3000
```

### Endpoints

#### Public Endpoints (no auth)

##### Root
```http
GET /
```
Endpoint di test per verificare che il server sia attivo.

**Response**
```json
{
  "message": "Ciao, Ruggine!"
}
```

---

#### Authentication Endpoints

##### Register
```http
POST /auth/register
Content-Type: application/json
```

**Body**
```json
{
  "username": "mario_rossi",
  "password": "SecurePass123!"
}
```

**Response** (201 Created)
```json
{
  "user_id": 1,
  "username": "mario_rossi"
}
```

##### Login
```http
POST /auth/login
Content-Type: application/json
```

**Body**
```json
{
  "username": "mario_rossi",
  "password": "SecurePass123!"
}
```

**Response** (200 OK)
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "user_id": 1,
    "username": "mario_rossi"
  }
}
```

---

#### User Endpoints (require auth)

Tutti questi endpoint richiedono header:
```http
Authorization: Bearer <token>
```

##### Search User by Username
```http
GET /users?username=mario
```

**Response** (200 OK)
```json
[
  {
    "user_id": 1,
    "username": "mario_rossi"
  },
  {
    "user_id": 5,
    "username": "mario_bianchi"
  }
]
```

##### Get User by ID
```http
GET /users/{user_id}
```

**Response** (200 OK)
```json
{
  "user_id": 1,
  "username": "mario_rossi"
}
```

##### Delete My Account
```http
DELETE /users/me
```

**Response** (204 No Content)

---

#### Chat Endpoints (require auth)

##### List My Chats
```http
GET /chats
```

**Response** (200 OK)
```json
[
  {
    "chat_id": 1,
    "title": "Team Project",
    "description": "Progetto università",
    "chat_type": "GROUP",
    "my_role": "OWNER",
    "unread_messages": 5,
    "last_message_at": "2025-11-05T14:30:00Z"
  },
  {
    "chat_id": 2,
    "title": null,
    "description": null,
    "chat_type": "PRIVATE",
    "my_role": "STANDARD",
    "unread_messages": 0,
    "last_message_at": "2025-11-04T10:15:00Z"
  }
]
```

##### Create Chat
```http
POST /chats
Content-Type: application/json
```

**Body (Group Chat)**
```json
{
  "title": "New Project Group",
  "description": "Chat per il nuovo progetto",
  "chat_type": "GROUP"
}
```

**Body (Private Chat)**
```json
{
  "chat_type": "PRIVATE",
  "other_user_id": 5
}
```

**Response** (201 Created)
```json
{
  "chat_id": 3,
  "title": "New Project Group",
  "description": "Chat per il nuovo progetto",
  "chat_type": "GROUP"
}
```

##### Get Chat Messages (require membership)
```http
GET /chats/{chat_id}/messages?limit=50&before_message_id=100
```

**Query Parameters**
- `limit` (optional): Numero massimo di messaggi (default: 50, max: 100)
- `before_message_id` (optional): ID messaggio per paginazione

**Response** (200 OK)
```json
[
  {
    "message_id": 99,
    "chat_id": 1,
    "sender_id": 2,
    "sender_username": "alice",
    "content": "Ciao a tutti!",
    "message_type": "USERMESSAGE",
    "created_at": "2025-11-05T14:30:00Z"
  },
  {
    "message_id": 98,
    "chat_id": 1,
    "sender_id": null,
    "sender_username": null,
    "content": "Alice è entrata nella chat",
    "message_type": "SYSTEMMESSAGE",
    "created_at": "2025-11-05T14:29:00Z"
  }
]
```

##### List Chat Members (require membership)
```http
GET /chats/{chat_id}/members
```

**Response** (200 OK)
```json
[
  {
    "user_id": 1,
    "username": "mario_rossi",
    "role": "OWNER",
    "joined_at": "2025-11-01T10:00:00Z"
  },
  {
    "user_id": 2,
    "username": "alice",
    "role": "ADMIN",
    "joined_at": "2025-11-02T11:30:00Z"
  },
  {
    "user_id": 3,
    "username": "bob",
    "role": "STANDARD",
    "joined_at": "2025-11-03T09:15:00Z"
  }
]
```

---

#### Membership Management (require membership)

##### Invite User to Chat (Admin/Owner only)
```http
POST /chats/{chat_id}/invite/{user_id}
```

**Response** (201 Created)
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

##### Update Member Role (Owner only)
```http
PATCH /chats/{chat_id}/members/{user_id}/role
Content-Type: application/json
```

**Body**
```json
{
  "new_role": "ADMIN"
}
```

**Response** (200 OK)
```json
{
  "message": "Role updated successfully"
}
```

##### Transfer Ownership (Owner only)
```http
PATCH /chats/{chat_id}/transfer_ownership
Content-Type: application/json
```

**Body**
```json
{
  "new_owner_id": 2
}
```

**Response** (200 OK)
```json
{
  "message": "Ownership transferred successfully"
}
```

##### Remove Member (Admin/Owner only)
```http
DELETE /chats/{chat_id}/members/{user_id}
```

**Response** (200 OK)
```json
{
  "message": "Member removed successfully"
}
```

##### Leave Chat
```http
POST /chats/{chat_id}/leave
```

**Response** (200 OK)
```json
{
  "message": "Successfully left the chat"
}
```

---

#### Invitation Endpoints

##### List Pending Invitations
```http
GET /invitations/pending
```

**Response** (200 OK)
```json
[
  {
    "invite_id": 10,
    "chat_id": 1,
    "chat_title": "Team Project",
    "invitee_id": 1,
    "invitee_username": "mario_rossi",
    "created_at": "2025-11-05T15:00:00Z"
  }
]
```

##### Respond to Invitation
```http
POST /invitations/{invite_id}/{action}
```

**Actions**: `accept` | `reject`

**Response** (200 OK)
```json
{
  "message": "Invitation accepted"
}
```

---

## WebSocket Protocol

### Connection

**Endpoint**
```
ws://127.0.0.1:3000/ws
```

**Headers** (required)
```
Authorization: Bearer <jwt_token>
```

### Configuration

- **Batch Interval**: 1000ms (max delay tra batch)
- **Batch Max Size**: 10 messaggi per batch
- **Rate Limit**: 10ms tra messaggi (max 100 msg/sec)
- **Timeout**: 300s di inattività prima della disconnessione
- **Broadcast Channel Capacity**: 100 messaggi

### Message Format

#### Client → Server: Send Message

```json
{
  "chat_id": 1,
  "sender_id": 2,
  "content": "Hello World!",
  "message_type": "USERMESSAGE"
}
```

**Validazione**:
- `chat_id`: required
- `sender_id`: must match authenticated user
- `content`: required, non-empty
- `message_type`: must be "USERMESSAGE" (system messages not allowed from clients)

#### Server → Client: New Message

```json
{
  "message_id": 150,
  "chat_id": 1,
  "sender_id": 2,
  "sender_username": "alice",
  "content": "Hello World!",
  "message_type": "USERMESSAGE",
  "created_at": "2025-11-05T15:30:00Z"
}
```

#### Server → Client: System Message

```json
{
  "message_id": 151,
  "chat_id": 1,
  "sender_id": null,
  "sender_username": null,
  "content": "Bob è entrato nella chat",
  "message_type": "SYSTEMMESSAGE",
  "created_at": "2025-11-05T15:31:00Z"
}
```

#### Server → Client: Error

```json
{
  "error": "You are not a member of this chat"
}
```

### Event Flow

1. **Client connects** → Server registers in `UserMap`
2. **Client sends message** → Server validates → Saves to DB → Broadcasts to chat members
3. **User joins chat** → System message broadcasted to all members
4. **Invitation accepted** → System message broadcasted
5. **Member removed** → System message broadcasted
6. **Client disconnects** → Server removes from `UserMap` and `ChatMap`

### WebSocket States

- **Connected**: Client autenticato e registrato
- **Rate Limited**: Client supera 100 msg/sec
- **Timeout**: Nessuna attività per 300s → disconnessione automatica
- **Error**: Messaggio malformato o non autorizzato

---

## Database Schema

### Entity Relationship Diagram

```
┌─────────────┐         ┌──────────────────┐         ┌─────────────┐
│   users     │────────┤ userchatmetadata ├────────│   chats     │
│             │         │                  │         │             │
│ user_id (PK)│         │ user_id (FK)     │         │ chat_id (PK)│
│ username    │         │ chat_id (FK)     │         │ title       │
│ password    │         │ role             │         │ description │
└──────┬──────┘         │ unread_messages  │         │ chat_type   │
       │                │ last_read_at     │         └──────┬──────┘
       │                │ joined_at        │                │
       │                └──────────────────┘                │
       │                                                    │
       │                ┌──────────────────┐                │
       └───────────────►│   invitations    │◄───────────────┘
       │                │                  │
       │                │ invite_id (PK)   │
       │                │ target_chat_id   │
       │                │ invited_id (FK)  │
       │                │ invitee_id (FK)  │
       │                │ state            │
       │                │ created_at       │
       │                └──────────────────┘
       │
       │                ┌──────────────────┐
       └───────────────►│    messages      │◄───────────────┐
                        │                  │                │
                        │ message_id (PK)  │                │
                        │ chat_id (FK)     ├────────────────┘
                        │ sender_id (FK)   │
                        │ content          │
                        │ message_type     │
                        │ created_at       │
                        └──────────────────┘
```

### Tables

#### `users`
- `user_id` INT (PK, AUTO_INCREMENT)
- `username` VARCHAR(50) (UNIQUE, NOT NULL)
- `password` VARCHAR(255) (NOT NULL) - bcrypt hashed

#### `chats`
- `chat_id` INT (PK, AUTO_INCREMENT)
- `title` VARCHAR(255) (NULL) - NULL for private chats
- `description` TEXT (NULL)
- `chat_type` ENUM('GROUP', 'PRIVATE') (NOT NULL)

#### `messages`
- `message_id` INT (PK, AUTO_INCREMENT)
- `chat_id` INT (FK → chats.chat_id, ON DELETE CASCADE)
- `sender_id` INT (FK → users.user_id, ON DELETE SET NULL) - NULL for system messages
- `content` TEXT (NOT NULL)
- `message_type` ENUM('USERMESSAGE', 'SYSTEMMESSAGE') (NOT NULL)
- `created_at` TIMESTAMP (NOT NULL)

#### `userchatmetadata`
- `user_id` INT (FK → users.user_id, ON DELETE CASCADE)
- `chat_id` INT (FK → chats.chat_id, ON DELETE CASCADE)
- `role` ENUM('OWNER', 'ADMIN', 'STANDARD') (NOT NULL)
- `unread_messages` INT (NOT NULL, DEFAULT 0)
- `last_read_at` TIMESTAMP (NULL)
- `joined_at` TIMESTAMP (NOT NULL)
- PRIMARY KEY (`user_id`, `chat_id`)

#### `invitations`
- `invite_id` INT (PK, AUTO_INCREMENT)
- `target_chat_id` INT (FK → chats.chat_id, ON DELETE CASCADE)
- `invited_id` INT (FK → users.user_id, ON DELETE CASCADE) - User being invited
- `invitee_id` INT (FK → users.user_id, ON DELETE CASCADE) - User who sent invitation
- `state` ENUM('PENDING', 'ACCEPTED', 'REJECTED') (NOT NULL, DEFAULT 'PENDING')
- `created_at` TIMESTAMP (NOT NULL)
- UNIQUE KEY (`target_chat_id`, `invited_id`, `state`)

### Indexes

- **users**: `username` (UNIQUE)
- **messages**: `chat_id`, `created_at`
- **userchatmetadata**: `(user_id, chat_id)` (PRIMARY)
- **invitations**: `target_chat_id`, `invited_id`, `invitee_id`

---

## Testing

### Run All Tests

```bash
cargo test
```

### Run Specific Test Suite

```bash
# Test autenticazione
cargo test api_auth

# Test chats
cargo test api_chats

# Test users
cargo test api_users
```

### Test Coverage

```bash
# Installare tarpaulin
cargo install cargo-tarpaulin

# Generare report coverage
cargo tarpaulin --out Html
```

### Test Fixtures

Il progetto include fixtures SQL per il testing:
- `fixtures/users.sql` - Utenti di test
- `fixtures/chats.sql` - Chat di test
- `fixtures/messages.sql` - Messaggi di test
- `fixtures/invitations.sql` - Inviti di test

---

## Struttura del Progetto

```
server/
├── Cargo.toml              # Dipendenze e configurazione progetto
├── .env                    # Variabili d'ambiente (non in git)
├── README.md              # Questa documentazione
├── tarpaulin-report.html  # Report coverage test
│
├── migrations/            # SQL migrations
│   └── 1_create_database.sql
│
├── fixtures/              # Test data
│   ├── users.sql
│   ├── chats.sql
│   ├── messages.sql
│   └── invitations.sql
│
├── scripts/               # Utility scripts
│   ├── create_tables.sql
│   └── grant_test_permissions.sql
│
├── src/
│   ├── main.rs           # Entry point & routing
│   ├── lib.rs            # Library exports
│   │
│   ├── core/             # Core functionality
│   │   ├── mod.rs
│   │   ├── config.rs     # Configuration management
│   │   ├── state.rs      # AppState & shared state
│   │   ├── auth.rs       # JWT middleware
│   │   └── error.rs      # Error handling
│   │
│   ├── entities/         # Domain models
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── chat.rs
│   │   ├── message.rs
│   │   ├── user_chat_metadata.rs
│   │   ├── invitation.rs
│   │   └── enums.rs      # ChatType, Role, InvitationState, MessageType
│   │
│   ├── dtos/             # Data Transfer Objects
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── chat.rs
│   │   ├── message.rs
│   │   ├── invitation.rs
│   │   ├── user_chat_metadata.rs
│   │   └── query.rs
│   │
│   ├── repositories/     # Database access layer
│   │   ├── mod.rs
│   │   ├── traits.rs     # CRUD traits
│   │   ├── user.rs
│   │   ├── chat.rs
│   │   ├── message.rs
│   │   ├── user_chat_metadata.rs
│   │   └── invitation.rs
│   │
│   ├── services/         # Business logic
│   │   ├── mod.rs        # Root handler
│   │   ├── auth.rs       # Login, register
│   │   ├── user.rs       # User management
│   │   ├── chat.rs       # Chat operations
│   │   └── membership.rs # Invitations, roles, members
│   │
│   └── ws/               # WebSocket handling
│       ├── mod.rs        # WS entry point
│       ├── connection.rs # Connection lifecycle
│       ├── event_handlers.rs # Message processing
│       ├── usermap.rs    # User→Sender mapping
│       └── chatmap.rs    # Chat→Users mapping
│
├── tests/                # Integration tests
│   ├── common/
│   │   └── mod.rs        # Test utilities
│   ├── api_auth.rs
│   ├── api_users.rs
│   └── api_chats.rs
│
└── target/               # Build output (gitignored)
```

### Layer Responsibilities

#### **Core Layer**
- Configurazione applicazione
- Gestione stato condiviso (AppState)
- Middleware di autenticazione
- Error handling centralizzato

#### **Entities Layer**
- Modelli di dominio puri
- Mapping 1:1 con tabelle database
- Tipi ENUM del dominio

#### **DTOs Layer**
- Oggetti per comunicazione API
- Validazione input con `validator`
- Trasformazioni entity ↔ DTO

#### **Repositories Layer**
- Accesso diretto al database
- Query SQLx compile-time checked
- Implementazione trait CRUD generici

#### **Services Layer**
- Business logic
- Orchestrazione operazioni complesse
- Validazione regole di business
- Composizione repository calls

#### **WebSocket Layer**
- Gestione connessioni persistenti
- Broadcasting messaggi
- Rate limiting
- Gestione timeout

---

## Security Features

### Authentication
- **Password Hashing**: bcrypt con salt automatico
- **JWT**: Token firmati con HS256
- **Token Expiration**: Configurabile (default: 24h)
- **Bearer Token**: Header `Authorization: Bearer <token>`

### Authorization
- **Middleware**: Verifica JWT su endpoint protetti
- **Membership Check**: Middleware per verificare appartenenza a chat
- **Role-Based**: Controlli Owner/Admin/Standard per operazioni privilegiate

### Input Validation
- **Validator**: Validazione automatica DTO con derive macro
- **SQL Injection**: Prevenuto da SQLx parametrized queries
- **XSS**: Sanitizzazione input lato client

### Rate Limiting
- **WebSocket**: Max 100 messaggi/secondo per client
- **Database Pool**: Max 1000 connessioni simultanee

---

## Performance

### Ottimizzazioni

1. **Connection Pooling**: SQLx pool con 1000 max connections
2. **Compile-time Queries**: SQLx verifica query a compile-time
3. **DashMap**: HashMap lock-free per WebSocket maps
4. **Message Batching**: Fino a 10 messaggi per batch ogni 1s
5. **Lazy DB Connections**: Retry automatico con backoff
6. **Zero-Copy**: Uso di reference dove possibile

### Benchmarks

```bash
# Build release ottimizzato
cargo build --release

# Dimensione binario
ls -lh target/release/server

# Profiling con flamegraph
cargo install flamegraph
cargo flamegraph
```

---

## Environment Examples

### Development
```env
DATABASE_URL=mysql://ruggine:ferro@127.0.0.1:3306/rugginedb
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
JWT_SECRET=dev_secret_key
APP_ENV=development
LOG_LEVEL=debug
MAX_DB_CONNECTIONS=100
DB_CONNECTION_LIFETIME_SECS=1
```

### Production
```env
DATABASE_URL=mysql://ruggine:secure_password@production-host:3306/rugginedb
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
JWT_SECRET=very_long_and_random_secret_key_here
APP_ENV=production
LOG_LEVEL=warn
MAX_DB_CONNECTIONS=1000
DB_CONNECTION_LIFETIME_SECS=3600
```

---

## Troubleshooting

### Database Connection Failed
```
✗ Failed to connect to database: Connection refused
```
**Soluzione**: Verificare che MySQL sia avviato e che le credenziali in `.env` siano corrette.

### JWT Invalid Token
```
{"error": "Invalid token"}
```
**Soluzione**: Token scaduto o `JWT_SECRET` non corrisponde. Rifare login.

### WebSocket Disconnected
```
Connection closed: timeout
```
**Soluzione**: Inattività per oltre 300 secondi. Riconnettersi.

### Permission Denied
```
{"error": "You must be an admin or owner"}
```
**Soluzione**: L'utente non ha il ruolo necessario per l'operazione.

---

## License

Questo progetto è sviluppato per scopi didattici nell'ambito del corso di Programmazione di Sistema 2024/2025.

