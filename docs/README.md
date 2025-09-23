# Documentazione Ufficiale – **Ruggine: App di Chat Testuale**

## 1. Introduzione
**Ruggine** è un’applicazione client/server sviluppata in **Rust** per la gestione di chat testuali.  
L’obiettivo è fornire un sistema efficiente, sicuro e multi–piattaforma che consenta comunicazioni sia private che di gruppo.

## 2. Obiettivi del Progetto
- Fornire un sistema di **messaggistica testuale** robusto e scalabile.
- Permettere la creazione di **gruppi di utenti**, accessibili solo tramite invito.
- Garantire **portabilità** su almeno due piattaforme tra Windows, Linux, MacOS, Android, ChromeOS e iOS.
- Ottimizzare **prestazioni** (CPU e dimensione binario).
- Implementare un sistema di **logging** periodico delle risorse utilizzate dal server.

## 3. Requisiti

### 3.1 Requisiti Funzionali
- **Gestione utenti**: iscrizione al primo avvio, login, logout, cancellazione account, ricerca utenti.
- **Chat private**: creazione, eliminazione, invio/ricezione messaggi, gestione ultimo messaggio visualizzato.
- **Chat di gruppo**: creazione, modifica, eliminazione, inviti, gestione ruoli (Owner, Admin, Standard), espulsioni, uscita volontaria.
- **Messaggi**: invio con limite dimensionale, recupero con paginazione, gestione metadati.
- **Inviti**: ricezione, accettazione o rifiuto invito.
- **Logging**: file di log aggiornato ogni 2 minuti con tempo di CPU del server.

### 3.2 Requisiti Non Funzionali
- Portabilità (almeno 2 piattaforme).
- Efficienza massima in CPU e memoria.
- Binario leggero, con dimensione riportata nel report.
- Sicurezza con **JWT bearer token** per autenticazione e autorizzazione.

## 4. API REST

### Autenticazione
- `POST /api/login`
- `POST /api/register`
- `DELETE /api/users/{id}`

### Utenti
- `GET /api/users?search=...`
- `GET /api/users/{id}/online`

### Chat
- `GET /api/chats`
- `POST /api/chats/private`
- `POST /api/chats/group`
- `GET /api/chats/{id}/messages?starting={id}&before=20&after=20`

### Gruppi
- `POST /api/chats/{id}/invite`
- `DELETE /api/chats/{id}/members/{user}`
- `POST /api/chats/{id}/leave`
- `PATCH /api/chats/{id}/members/{user}/role`
- `PATCH /api/chats/{id}/transfer-ownership`

### Inviti
- `GET /api/invitations`
- `POST /api/invitations/{id}`

## 5. WebSocket
- **Endpoint**: `WS /ws/chat`
- Invio messaggi:
  ```json
  {
    "type": "message",
    "chat_id": "uuid",
    "content": "string",
    "createdAt": "datetime"
  }
  ```
- Eliminazione messaggi:
  ```json
  {
    "type": "delete",
    "chat_id": "uuid",
    "message_id": "uuid"
  }
  ```

## 6. Modellazione

### Classi principali
- `User`, `Message`, `PrivateChat`, `GroupChat`, `Invitation`, `OnlineUsers`
- `UserChatMetadata` (gestione messaggi consegnati/visibili)

### Enum
- `MessageType { UserMessage, SystemMessage }`
- `Role { Owner, Admin, Standard }`
- `InvitationStatus { Pending, Accepted, Rejected }`

### UML



![Diagramma concettuale](server.png)

## 7. Logging e Monitoraggio
- File di log generato dal server ogni 2 minuti.
- Include tempo CPU e utilizzo risorse.
- Libreria: **sysinfo + tracing**.

## 8. Misure di Prestazioni
- Verrà riportata la **dimensione del file eseguibile** del server e del client.
- Monitoraggio CPU e memoria relativamente al server.

