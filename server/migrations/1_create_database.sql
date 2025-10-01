-- MySQL Migration for Ruggine Chat App

/* ===============================
   Us/* ===============================
   Note:
   - Le chat private sono gestite tramite il campo type = 'Private'
   - Tutte le FK hanno ON DELETE coerenti con l'uso:
       * cancellare una chat elimina messaggi/metadati/inviti relativi
       * cancellare un utente elimina i suoi metadati e messaggi
   =============================== */=============================== */
CREATE TABLE Users (
                     id          INT PRIMARY KEY AUTO_INCREMENT,
                     username    VARCHAR(255) UNIQUE NOT NULL,
                     passwordHash TEXT NOT NULL
);

/* ===============================
   Chats  (Groups and Private)
   =============================== */
CREATE TABLE Chats (
                     id          INT PRIMARY KEY AUTO_INCREMENT,
                     title       VARCHAR(255),
                     description TEXT,
                     type        ENUM('Private','Group') NOT NULL DEFAULT 'Private'
);

/* ===============================
   Messages
   =============================== */
CREATE TABLE Messages (
                        id         INT PRIMARY KEY AUTO_INCREMENT,
                        chatId     INT NOT NULL,
                        senderId   INT NOT NULL,
                        content    TEXT NOT NULL,
                        type       ENUM('userMessage','systemMessage') NOT NULL DEFAULT 'userMessage',
                        createdAt  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

                        FOREIGN KEY (chatId)   REFERENCES Chats(id)  ON DELETE CASCADE,
                        FOREIGN KEY (senderId) REFERENCES Users(id)  ON DELETE CASCADE
);

-- Create indexes separately
CREATE INDEX idx_Messages_chat_createdAt ON Messages(chatId, createdAt DESC);
CREATE INDEX idx_Messages_sender ON Messages(senderId);

/* ===============================
   UserChatMetadata
   =============================== */
CREATE TABLE UserChatMetadata (
                                 userId        INT NOT NULL,
                                 chatId        INT NOT NULL,
                                 lastDelivered INT,   -- FK a Messages(id)
                                 deliverFrom   INT,   -- FK a Messages(id)
                                 role          ENUM('owner','admin','member') NOT NULL DEFAULT 'member',
                                 
                                 PRIMARY KEY (userId, chatId),

                                 FOREIGN KEY (userId)        REFERENCES Users(id)    ON DELETE CASCADE,
                                 FOREIGN KEY (chatId)        REFERENCES Chats(id)    ON DELETE CASCADE,
                                 FOREIGN KEY (lastDelivered) REFERENCES Messages(id) ON DELETE SET NULL,
                                 FOREIGN KEY (deliverFrom)   REFERENCES Messages(id) ON DELETE SET NULL
);

-- Create indexes separately
CREATE INDEX idx_UCM_user ON UserChatMetadata(userId);
CREATE INDEX idx_UCM_chat ON UserChatMetadata(chatId);

/* ===============================
   Invitations (solo per gruppi)
   =============================== */
CREATE TABLE Invitations (
                           id             INT PRIMARY KEY AUTO_INCREMENT,
                           groupId        INT NOT NULL,  -- = Chats.id
                           invitedUserId  INT NOT NULL,  -- = Users.id
                           invitedById    INT,           -- = Users.id (nullable, ON DELETE SET NULL)
                           status         ENUM('pending','accepted','rejected') NOT NULL DEFAULT 'pending',

                           FOREIGN KEY (groupId)       REFERENCES Chats(id)  ON DELETE CASCADE,
                           FOREIGN KEY (invitedUserId) REFERENCES Users(id)  ON DELETE CASCADE,
                           FOREIGN KEY (invitedById)   REFERENCES Users(id)  ON DELETE SET NULL
);

-- Create indexes separately  
-- Evita inviti duplicati per lo stesso utente/gruppo con stesso status
CREATE UNIQUE INDEX uq_Invitations_group_user_status ON Invitations(groupId, invitedUserId, status);
CREATE INDEX idx_Invitations_group ON Invitations(groupId);
CREATE INDEX idx_Invitations_invited_user ON Invitations(invitedUserId);


/* ===============================
   Note:
   - Le chat private NON sono in questo schema. Per i DM usa
     tabelle dedicate (es. DirectConversations/DirectMessages) oppure gestiscile lato app.
   - Tutte le FK hanno ON DELETE coerenti con l’uso:
       * cancellare una chat elimina messaggi/metadati/inviti relativi
       * cancellare un utente elimina i suoi metadati e messaggi (se vuoi conservarli, cambia ON DELETE)
   =============================== */