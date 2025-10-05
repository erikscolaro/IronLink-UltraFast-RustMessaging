# ## 1. Introduzione
**Ruggine** è un'applicazione client/server sviluppata in **Rust** per la gestione di chat testuali.  
L'obiettivo è fornire un sistema efficiente, sicuro e multi–piattaforma che consenta comunicazioni sia private che di gruppo.

## 2. Stato Corrente dell'Implementazione

### 2.1 Server [x] (Parzialmente Implementato)
- **Architettura**: REST API con Axum framework
- **Database**: MySQL con SQLx per ORM  
- **Autenticazione**: JWT Bearer Token implementato
- **Struttura modulare**: Repositories, Services, Models, Error handling
- **Routing**: Endpoints REST definiti (implementazione in corso)

### 2.2 Client [ ] (Non Implementato)
- Attualmente solo stub "Hello, world!"
- Struttura Rust pronta per lo sviluppo

## 3. Obiettivi del Progettoentazione Ufficiale – **Ruggine: App di Chat Testuale**

## 1. Introduzione
**Ruggine** è un’applicazione client/server sviluppata in **Rust** per la gestione di chat testuali.  
L’obiettivo è fornire un sistema efficiente, sicuro e multi–piattaforma che consenta comunicazioni sia private che di gruppo.

## 2. Obiettivi del Progetto
- Fornire un sistema di **messaggistica testuale** robusto e scalabile.
- Permettere la creazione di **gruppi di utenti**, accessibili solo tramite invito.
- Garantire **portabilità** su almeno due piattaforme tra Windows, Linux, MacOS, Android, ChromeOS e iOS.
- Ottimizzare **prestazioni** (CPU e dimensione binario).
- Implementare un sistema di **logging** periodico delle risorse utilizzate dal server.

## 4. Requisiti

### 4.1 Requisiti Funzionali (Stato di implementazione)
- **Gestione utenti**: [x] Struttura implementata, [~] Login parziale, [ ] Altri endpoint da completare
- **Chat unificate**: [x] Modello unificato Chat (Group/Private), [ ] Logica di business da implementare  
- **Messaggi**: [x] Modello implementato, [ ] Invio/ricezione da implementare
- **Inviti**: [x] Modello implementato, [ ] Logica di business da implementare
- **Logging**: [ ] Non implementato

### 4.2 Requisiti Non Funzionali
- Portabilità (almeno 2 piattaforme).
- Efficienza massima in CPU e memoria.
- Binario leggero, con dimensione riportata nel report.
- Sicurezza con **JWT bearer token** per autenticazione e autorizzazione. [x] **Implementato**

## 5. API REST (Implementazione Corrente)

### 5.1 Endpoints Implementati
#### Autenticazione
- `POST /auth/login` [x] **Implementato** - Login utente con JWT
- `POST /auth/logout` [~] **Struttura pronta** - Logout utente  
- `POST /auth/register` [~] **Struttura pronta** - Registrazione utente

#### Utenti 
- `GET /users` [~] **Struttura pronta** - Ricerca utenti (query param `?search=...`)
- `GET /users/{id}` [~] **Struttura pronta** - Informazioni utente specifico
- `DELETE /users/me` [~] **Struttura pronta** - Cancellazione del proprio account

#### Chat Unificate (Group + Private)
- `GET /chats` [~] **Struttura pronta** - Lista delle chat dell'utente
- `POST /chats` [~] **Struttura pronta** - Creazione nuova chat (se privata evita la creazione di duplicati)
- `GET /chats/{id}/messages` [~] **Struttura pronta** - Messaggi di una chat
- `GET /chats/{id}/members` [~] **Struttura pronta** - Lista membri di una chat

#### Gestione Gruppo
- `POST /chats/{id}/invite` [~] **Struttura pronta** - Invito a gruppo
- `DELETE /chats/{id}/members/{id}` [~] **Struttura pronta** - Rimozione membro
- `POST /chats/{id}/leave` [~] **Struttura pronta** - Uscita da gruppo
- `PATCH /chats/{id}/members/{id}/role` [~] **Struttura pronta** - Cambio ruolo membro
- `PATCH /chats/{id}/members/{id}/transfer-ownership` [~] **Struttura pronta** - Trasferimento ownership

### 5.2 Inviti Rimossi
Gli inviti sono ora gestiti tramite **messaggi di sistema** nelle chat private anziché endpoint dedicati.

## 6. WebSocket (Implementazione Futura)
- **Nota**: La messaggistica in tempo reale verrà implementata successivamente
- **Endpoint pianificato**: `WS /ws/chat`
- **Funzionalità future**:
  - Invio messaggi in tempo reale
  - Notifiche di typing  
  - Gestione utenti online (OnlineUsers)

## 7. Modellazione (Implementazione Corrente)

### 7.1 Strutture Dati Implementate
#### Modelli Core
- `User` - Utente del sistema (ID: u32, username, password hash)
- `Message` - Messaggio in chat (ID: u32, chat_id, sender_id, content, timestamp, tipo)
- `Chat` - Chat unificata (ID: u32, titolo, descrizione, tipo: Group/Private)
- `UserChatMetadata` - Metadati utente-chat (ruoli, messaggi visualizzati)
- `Invitation` - Invito a gruppo (ID: u32, chat_id, utenti coinvolti, stato)

#### Repository Pattern
- Trait `Crud<T, Id>` per operazioni CRUD generiche
- Repository specifici: `UserRepository`, `MessageRepository`, `ChatRepository`, etc.
- Database: SQLite con SQLx

### 7.2 Enum Implementate
- `MessageType { UserMessage, SystemMessage }`
- `UserRole { Owner, Admin, Standard }`
- `InvitationStatus { Pending, Accepted, Rejected }`
- `ChatType { Group, Private }`

### 7.3 Architettura Sistema
- `AppState` - Stato condiviso dell'applicazione (repositories + JWT secret)
- `Claims` - Payload JWT per autenticazione
- `AppError` - Gestione errori unificata

### 7.4 Schema Database (MySQL)
- Tabelle: `Users`, `Chats`, `Messages`, `UserChatMetadata`, `Invitations`
- Relazioni con foreign keys e cascading
- Indici per performance su query frequenti

### UML

![Diagramma concettuale](server.png)

## 8. Tecnologie Utilizzate

### 8.1 Server (Rust)
- **Framework Web**: Axum (async, performante)
- **Database**: MySQL con SQLx (compile-time checked queries)
- **Autenticazione**: jsonwebtoken + bcrypt
- **Serialization**: serde (JSON)
- **Async Runtime**: tokio
- **Config**: dotenv per variabili d'ambiente

### 8.2 Client (Rust - In sviluppo)
- Struttura base creata ma non implementata

## 9. Logging e Monitoraggio [ ] (Non Implementato)
- **Pianificato**: File di log generato dal server ogni 2 minuti
- **Librerie da usare**: sysinfo + tracing
- **Metriche**: Tempo CPU, utilizzo memoria, connessioni attive

## 10. Prossimi Passi per lo Sviluppo

### 10.1 Priorità Alta
1. **Completare implementazione repository** - Sostituire `todo!()` con logica SQLx
2. **Implementare services layer** - Logica di business per tutti gli endpoints
3. **Testing database** - Setup test con database isolati
4. **WebSocket server** - Messaggistica in tempo reale

### 10.2 Priorità Media  
1. **Client implementation** - Interface utente
2. **Logging system** - Monitoraggio risorse server
3. **Error handling migliorato** - Messaggi d'errore più specifici
4. **Validation layer** - Validazione input utente

### 10.3 Note di Sviluppo
- **Database**: Schema MySQL completo e migrato
- **Architecture**: Clean separation between layers (models, repositories, services)
- **Security**: JWT implementato, password hashing con bcrypt
- **Performance**: SQLx per query compile-time checked, connection pooling configurato

