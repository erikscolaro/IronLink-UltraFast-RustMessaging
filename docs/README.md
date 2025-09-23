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

![Diagramma concettuale](https://www.plantuml.com/plantuml/uml/fLTBR-Cs4BxhLmnooADracwFXc5WiG7Q00sos6pQeq2BOvkDH0eaPSDjzx_ly2LHihC9q4iYKJF3UNpppRYgcJ7mR533lmN0puBFuRaJt3rtT2fPWtDuibh8ZJjhINolD2-tp6pp3si9EzHYVIYJoEMvmNezW_G-XtDZzHLSBs6bL1sLgYsJ6yoKciv2K9Iu7rrp5LNe_7BgMtxDM9fZZHO7kzw1BsYwsYukhvP5yQqeAf-5HraBYIuN9YIZKb9YPOMIPWw_aoSFh5sux_Ty1d_XwabLBsW_zLBBFYQiNagnBfMtc1BXV2vTWi-3ZMagDcZBuSeVQZssJcHn_WCd6ESZQqDUUuif52SsFnLGog76HktmBNF9mI7nWY1HAbqQgtIVXiPivl20nh0D-c3kWk0X52LghxHYl6B6bbQL11ARrDrpsM2peZBDuK-iOaMMqV9vCZqKDfuS8zpRoWyCIfwIasIKj2XjZdLxQB4FuIkzU3cDp28l6OdUqTDNqU3YsLRB3BMZ-sOmkSqN8h_BPoTU4wqOPBhQJWsTqlj36Bj8oN1EN81KrbSBAbRNo4SDKZQKUz7MEDZklEtukMjT8PfWhXIogZkEKUWgMQUiH0a3J2E8Xc32LZM2aXMwU2rSIXLjtY8NTjAcKVq-MPuqwxIaTnyVVLN91cIXX8z7neu5_q_Cv4i2rlLrvb1Hi1KH7QiiV2Api5ZGYx3BPS2VBwBalPDP9nrrHXRvG14fmvSLfHFrF971GugMEc65Q-gu_bDdn3z8nsoTAuHJk0fTb275tIHCRB8Z7dfETPbGL-p98Ey5V7EIzFATLDnNzxCMEseXdxwZfiqcm8VK0ExbryorpC1WzJUjklQqJIjiFPfp74sbXS_JPCgvkqVY3Yv7mZDGUpbDME_0UFPvQzF1DxhYQgoGtJTLE-7AXCHTgd8jpmNQ7mTNOMQPTWKTZaw3aM0jpTenl0lLR4MRssrH4mOROiJL8PN9UNWYBoFB5w0L4KO8rS0jzDOzfT0n9-u28VtuSjbyGCiuisoezFHBueWHXiFHT0JGKUscJfnUhjrZkoGaUtI4qRRMgZdrtKKmXo_qrxzVf5a8Je5QYxXI4ZjAOGLb_p03koLsF1YBJI9GQ3fZgO_3aE2nemDz-xutNcdZ81KlKNIWzy8WvFu1LsWyzM_P3e71ZjLKPCTXO1KOAhQ5lR3Ru7hMNHaXJlXncg6UUdmvdwZ-lR6kfG9WETQOaDw0XgbQ7iZRIh4zNFRVWyIXIiSeV1zXXRox3Z9HbWOTSAR0ut00vBtS2SiSJ8wvnvTmj7889qtkJk7QVrWTnbiBcUve6HfkUV8d6MEDBvb1iAJ7hvZrP2pYHS2XtsX1hA4v1EHKIciqhPBSUF1uC4OhsKne2Rn4GQNeFJDfdlHNzdFUZOiX1Cy57giXRuHLwWRk6YVsWiix6VTNDsYtYlVprPOX9paC1Xyo93iSl9Q-KzoK1KBtIzTMv9N6dNf1VaifeO0TuE2WEktsChS926dVFm4Bb5sJBvSUCOxHXatmoLD5UlyMTHoIlV2ZOqpw_v4tGZfnIwJ9DEybfv5whS-XLawlgh2rVa7yWd_JnezUzOBUwX0xiR4fZbZsx9URcv_EQ1jQmjc7i_2hSaxvgPNS65gbtjJ8vxvR9ODKITL-KtKOxXF5VaaxxLtOmLyJ6ZiaPDc60_-Bv3Mbt1LI2Qtd5Ohy9wQYhJ5gHb_SqlXYYc8ma5iEZncJp8dHJTvQHVRHEJ89DQULTugk3gy9va76VI7C8eA9ttTf-QVDkg5P5PIPrcflijXkRlvR-WsvoBCyd7u7)

## 7. Logging e Monitoraggio
- File di log generato dal server ogni 2 minuti.
- Include tempo CPU e utilizzo risorse.
- Libreria: **sysinfo + tracing**.

## 8. Misure di Prestazioni
- Verrà riportata la **dimensione del file eseguibile** del server e del client.
- Monitoraggio CPU e memoria relativamente al server.

