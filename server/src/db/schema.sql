PRAGMA foreign_keys = ON;

/* ===============================
   Users
   =============================== */
CREATE TABLE Users (
  id          INTEGER PRIMARY KEY,
  username    TEXT UNIQUE NOT NULL,
  passwordHash TEXT NOT NULL,
);

/* ===============================
   Chats  (solo gruppi)
   =============================== */
CREATE TABLE Chats (
  id          INTEGER PRIMARY KEY,
  title       TEXT NOT NULL,
  description TEXT,

  -- tutte le chat sono gruppi: titolo obbligatorio e non vuoto
);

/* ===============================
   Messages
   =============================== */
CREATE TABLE Messages (
  id         INTEGER PRIMARY KEY,
  chatId     INTEGER NOT NULL,
  senderId   INTEGER NOT NULL,
  content    TEXT NOT NULL,
  type       TEXT NOT NULL DEFAULT 'userMessage' CHECK (type IN ('userMessage','systemMessage')),
  createdAt  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY (chatId)   REFERENCES Chats(id)  ON DELETE CASCADE,
  FOREIGN KEY (senderId) REFERENCES Users(id)  ON DELETE CASCADE
);

CREATE INDEX idx_Messages_chat_createdAt ON Messages(chatId, createdAt DESC);
CREATE INDEX idx_Messages_sender         ON Messages(senderId);

/* ===============================
   UserChatMetadata
   =============================== */
CREATE TABLE UserChatMetadata (
  userId        INTEGER NOT NULL,
  chatId        INTEGER NOT NULL,
  lastDelivered INTEGER,   -- FK a Messages(id)
  deliverFrom   INTEGER,   -- FK a Messages(id)
  role          TEXT CHECK (role IN ('owner','admin','member')),
  PRIMARY KEY (userId, chatId),

  FOREIGN KEY (userId)        REFERENCES Users(id)    ON DELETE CASCADE,
  FOREIGN KEY (chatId)        REFERENCES Chats(id)    ON DELETE CASCADE,
  FOREIGN KEY (lastDelivered) REFERENCES Messages(id) ON DELETE SET NULL,
  FOREIGN KEY (deliverFrom)   REFERENCES Messages(id) ON DELETE SET NULL
);

CREATE INDEX idx_UCM_user ON UserChatMetadata(userId);
CREATE INDEX idx_UCM_chat ON UserChatMetadata(chatId);

/* ===============================
   Invitations (solo per gruppi)
   =============================== */
CREATE TABLE Invitations (
  id             INTEGER PRIMARY KEY,
  groupId        INTEGER NOT NULL,  -- = Chats.id
  invitedUserId  INTEGER NOT NULL,  -- = Users.id
  invitedById    INTEGER,           -- = Users.id (nullable, ON DELETE SET NULL)
  status        TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','accepted','rejected')),

  FOREIGN KEY (groupId)       REFERENCES Chats(id)  ON DELETE CASCADE,
  FOREIGN KEY (invitedUserId) REFERENCES Users(id)  ON DELETE CASCADE,
  FOREIGN KEY (invitedById)   REFERENCES Users(id)  ON DELETE SET NULL
);

-- Evita inviti duplicati per lo stesso utente/gruppo con stesso status
CREATE UNIQUE INDEX uq_Invitations_group_user_status
  ON Invitations(groupId, invitedUserId, status);

CREATE INDEX idx_Invitations_group        ON Invitations(groupId);
CREATE INDEX idx_Invitations_invited_user ON Invitations(invitedUserId);


/* ===============================
   Note:
   - Le chat private NON sono in questo schema. Per i DM usa
     tabelle dedicate (es. DirectConversations/DirectMessages) oppure gestiscile lato app.
   - Tutte le FK hanno ON DELETE coerenti con l’uso:
       * cancellare una chat elimina messaggi/metadati/inviti relativi
       * cancellare un utente elimina i suoi metadati e messaggi (se vuoi conservarli, cambia ON DELETE)
   =============================== */
