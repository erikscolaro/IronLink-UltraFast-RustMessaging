use crate::entities::{Chat, IdType, Invitation, Message, User, UserChatMetadata, UserRole, MessageType, ChatType, InvitationStatus};
use sqlx::{Error, MySqlPool};
use chrono::{DateTime, Utc};

// alias di tipo per il pool, per semplificare lo switch in caso in cui vogliamo usare un altro db
pub type PoolType = MySqlPool;
//***************************** TRATTI ****************************//

/*
 * Ci è stra utile per uniformare le operazioni crud tra di loro
 * inoltre ci da anche una struttura di default per gli eventuali altri metodi crud
 */

/// Trait per operazioni CRUD generiche
pub trait Crud<T, Id> {
    /// Crea un nuovo record e lo restituisce
    async fn create(&self, item: &T) -> Result<T, sqlx::Error>;

    /// Legge un record tramite ID
    async fn read(&self, id: &Id) -> Result<Option<T>, sqlx::Error>;

    /// Aggiorna un record esistente
    async fn update(&self, item: &T) -> Result<T, sqlx::Error>;

    /// Cancella un record tramite ID
    async fn delete(&self, id: &Id) -> Result<(), sqlx::Error>;
}

// ************************* REPOSITORY ************************* //

/*
   hey tu!
   Leggimi :D
   Ti risparmio un po' di dolore ( non vedere https://docs.rs/sqlx/latest/sqlx/macro.query.html )
   Quando devi fare query con sqlx, ci sono due modi: uno che permette di controllare staticamente
   che la query sia corretta nel senso che lo schema che abbiamo scritto coincida con quello del db
   (ovvero, in fase di compilazione, quella che ci piace perchè vogliamo essere sicuri che vada tutto bene)
   e uno che fa questo check in run-time (che ci fa schifo, quindi evito proprio di parlarne).
   Quindi, come si scrive una query? con la bellissima macro:
   sqlx::query!("SELECT id, name FROM users WHERE id = ?", 1)
   Per evitare di diventare scemi con le maiuscole, si possono scrivere anche in minusolo le keyword
   e si possono scrivere query anche complesse, tipo quelle annidate se serve!
   Ci sarebbe anche un altro modo in realtà di scrivere la query:
   sqlx::query!(
       "select * from (select (1) as id, 'Herp Derpinson' as name) accounts where id = ?",
       1i32
   )
   Ovvero inserendo direttamente dentro la stringa il valore, ma non si fam, anche se un valore sappiamo
   Rimanere sempre quello, comunque lo mettiamo con la sintassi che abbiamo visto prima.
   Ovviamente non è finita qui, la query segue il builder pattern con lazy execution -> concateniamo con la dot notation
   le varie operazioni supplementari, tipo: quanti risultati vogliamo ? uno solo, uno o più, almeno uno ...
   ecco le opzioni:
   Number of Rows	Method to Call*	Returns	Notes
   None†	        .execute(...).await	        sqlx::Result<DB::QueryResult>	            For INSERT/UPDATE/DELETE without RETURNING.
   Zero or One	    .fetch_optional(...).await	sqlx::Result<Option<{adhoc struct}>>	    Extra rows are ignored.
   Exactly One	    .fetch_one(...).await	    sqlx::Result<{adhoc struct}>	            Errors if no rows were returned. Extra rows are ignored. Aggregate queries, use this.
   At Least One	.fetch(...)	                impl Stream<Item = sqlx::Result<{adhoc struct}>>	Call .try_next().await to get each row result.
   Multiple	    .fetch_all(...)	            sqlx::Result<Vec<{adhoc struct}>>
   abbiamo scritto la query, ma ricordiamoci che è un metodo async quindi dobbiamo concludere con
   await e visto che abbiamo progettato bene le firme, addirittura con await? in modo che l'errore viene propagato al service
   o alla route che poi lo va a gestire restituendo al client l'adeguato codice errore.
   AH! Volevi fosse così semplice! E invece no, perchè si ritorniamo un result, ma questo result deve essere o l'oggetto
   GIA' parsato, oppure l'errore di sqlx :D

   In questi casi (quindi nella create, update, o nella read) dobbiamo usare al posto di query! -> query_as!
   Questa funzione magica ci fa già il parsing in automatico di quello che ci serve
   Sintassi ( molto simile ) :
   sqlx::query_as!(
       User, // tipo in output
       "SELECT id, name, email FROM users WHERE id = ?", //query con placeholder
       1 //valori
   )
   .fetch_one(&pool) //prendi esattamente uno da cosa? dal pool di connessioni della repo!
   .await?;

   Nota : visto che la compilazione è statica a compile time, se il database non è connesso correttamente o il server
   che contiene mysql non è attivo, il riusltaot è che query_as! e query! danno errore


*/
//TODO: "bisogna aggiungere alle definizioni dei models i tipi che vengono usati nel database, chiarire questo aspetto"<

//MOD -> possibile modifica
// Controllare se in alcuni casi non vogliamo l'oggetto come risultato ma solo un valore, e viceversa
//Per le crud, non sempre ritorno l'oggetto, quindi servirà poi fare una lettura successiva in services oppure scriverlo nel messaggio ok()
// USER REPO
pub struct UserRepository {
    connection_pool: PoolType,
}

impl UserRepository {
    pub fn new(connection_pool: PoolType) -> UserRepository {
        Self { connection_pool }
    }

///considero l'username univoco
    /// Find user by exact username match
    /// For partial username search, use search_by_username_partial
    pub async fn find_by_username(&self, username: &String) -> Result<Option<User>, Error> {
        let user = sqlx::query_as!(
            User,
            "SELECT id as user_id, username, passwordHash as password FROM Users WHERE username = ?",
            username
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        Ok(user)
    }

    /// Search users by partial username match (for search functionality)
    pub async fn search_by_username_partial(&self, username_pattern: &String) -> Result<Vec<User>, Error> {
        let pattern = format!("%{}%", username_pattern);
        let users = sqlx::query_as!(
            User,
            "SELECT id as user_id, username, passwordHash as password FROM Users WHERE username LIKE ? LIMIT 50",
            pattern
        )
        .fetch_all(&self.connection_pool)
        .await?;
        
        Ok(users)
    }

}

impl Crud<User, IdType> for UserRepository {
    async fn create(&self, item: &User) -> Result<User, Error> {
        // Insert user and get the ID using MySQL syntax
        let result = sqlx::query!(
            "INSERT INTO Users (username, passwordHash) VALUES (?, ?)",
            item.username,
            item.password
        )
        .execute(&self.connection_pool)
        .await?;

        // Get the last inserted ID
        let new_id = result.last_insert_id() as IdType;

        // Return the created user with the new ID
        Ok(User {
            user_id: new_id,
            username: item.username.clone(),
            password: item.password.clone(),
        })
    }

    async fn read(&self, id: &IdType) -> Result<Option<User>, Error> {
        let user = sqlx::query_as!(
            User,
            "SELECT id as user_id, username, passwordHash as password FROM Users WHERE id = ?",
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        Ok(user)
    }

    async fn update(&self, item: &User) -> Result<User, Error> {
        sqlx::query!(
            "UPDATE Users SET username = ?, passwordHash = ? WHERE id = ?",
            item.username,
            item.password,
            item.user_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        // Return the updated user
        Ok(item.clone())
    }

    /// Soft delete user by setting username to "Deleted User" and clearing password ""
    /// This preserves message history while anonymizing the user
    async fn delete(&self, user_id: &IdType) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE Users SET username = 'Deleted User', passwordHash = '' WHERE id = ?",
            user_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }
}

// MESSAGE REPO
pub struct MessageRepository {
    connection_pool: PoolType,
}

impl MessageRepository {
    pub fn new(connection_pool: PoolType) -> Self {
        Self { connection_pool }
    }

    /// Get all messages for a specific chat, ordered by creation time
    pub async fn get_messages_by_chat_id(&self, chat_id: &IdType) -> Result<Vec<Message>, Error> {
        let messages = sqlx::query_as!(
            Message,
            r#"
            SELECT 
                id as message_id, 
                chatId as chat_id, 
                senderId as sender_id, 
                content, 
                createdAt as created_at,
                type as "message_type: MessageType"
            FROM Messages 
            WHERE chatId = ? 
            ORDER BY createdAt ASC
            "#,
            chat_id
        )
        .fetch_all(&self.connection_pool)
        .await?;
        
        Ok(messages)
    }

//MOD: imposta timestamp  after_timestamp = current_time-20minuti?????
    /// Get messages for a chat after a specific timestamp (for pagination)
    pub async fn get_messages_after_timestamp(
        &self, 
        chat_id: &IdType, 
        after_timestamp: &DateTime<Utc>
    ) -> Result<Vec<Message>, Error> {
        let messages = sqlx::query_as!(
            Message,
            r#"
            SELECT 
                id as message_id, 
                chatId as chat_id, 
                senderId as sender_id, 
                content, 
                createdAt as created_at,
                type as "message_type: MessageType"
            FROM Messages 
            WHERE chatId = ? AND createdAt > ?
            ORDER BY createdAt ASC
            "#,
            chat_id,
            after_timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?;
        
        Ok(messages)
    }

//MOD: se vogliomano caricare solo fino a 50 (limit) messaggi
    /// Get messages for a chat with limit (for pagination)
    pub async fn get_messages_with_limit(
        &self, 
        chat_id: &IdType, 
        limit: i64,
        offset: i64
    ) -> Result<Vec<Message>, Error> {
        let messages = sqlx::query_as!(
            Message,
            r#"
            SELECT 
                id as message_id, 
                chatId as chat_id, 
                senderId as sender_id, 
                content, 
                createdAt as created_at,
                type as "message_type: MessageType"
            FROM Messages 
            WHERE chatId = ? 
            ORDER BY createdAt DESC
            LIMIT ? OFFSET ?
            "#,
            chat_id,
            limit,
            offset
        )
        .fetch_all(&self.connection_pool)
        .await?;
        
        Ok(messages)
    }
}

impl Crud<Message, IdType> for MessageRepository {
    async fn create(&self, item: &Message) -> Result<Message, Error> {
        // Insert message using MySQL syntax
        let result = sqlx::query!(
            r#"
            INSERT INTO Messages (chatId, senderId, content, type, createdAt) 
            VALUES (?, ?, ?, ?, ?)
            "#,
            item.chat_id,
            item.sender_id,
            item.content,
            item.message_type as MessageType,
            item.created_at
        )
        .execute(&self.connection_pool)
        .await?;

        // Get the last inserted ID
        let new_id = result.last_insert_id() as IdType;

        // Return the created message with the new ID
        Ok(Message {
            message_id: new_id,
            chat_id: item.chat_id,
            sender_id: item.sender_id,
            content: item.content.clone(),
            created_at: item.created_at,
            message_type: item.message_type.clone(),
        })
    }

    async fn read(&self, id: &IdType) -> Result<Option<Message>, Error> {
        let message = sqlx::query_as!(
            Message,
            r#"
            SELECT 
                id as message_id, 
                chatId as chat_id, 
                senderId as sender_id, 
                content, 
                createdAt as created_at,
                type as "message_type: MessageType"
            FROM Messages 
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        Ok(message)
    }

    async fn update(&self, item: &Message) -> Result<Message, Error> {
        sqlx::query!(
            r#"
            UPDATE Messages 
            SET chatId = ?, senderId = ?, content = ?, type = ?, createdAt = ?
            WHERE id = ?
            "#,
            item.chat_id,
            item.sender_id,
            item.content,
            item.message_type as MessageType,
            item.created_at,
            item.message_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        // Return the updated message
        Ok(item.clone())
    }

    async fn delete(&self, id: &IdType) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM Messages WHERE id = ?",
            id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }
}

// USERCHATMETADATA REPO
pub struct UserChatMetadataRepository {
    connection_pool: PoolType,
}

impl UserChatMetadataRepository {
    pub fn new(connection_pool: PoolType) -> Self {
        Self { connection_pool }
    }

    /// Get all members of a specific chat
    pub async fn get_members_by_chat(&self, chat_id: &IdType) -> Result<Vec<UserChatMetadata>, Error> {
        let metadata_list = sqlx::query_as!(
            UserChatMetadata,
            r#"
            SELECT 
                userId as user_id,
                chatId as chat_id,
                role as "user_role: UserRole",
                NOW() as "member_since: DateTime<Utc>",
                '1970-01-01 00:00:00' as "messages_visible_from: DateTime<Utc>",
                NOW() as "messages_received_until: DateTime<Utc>"
            FROM UserChatMetadata 
            WHERE chatId = ?
            "#,
            chat_id
        )
        .fetch_all(&self.connection_pool)
        .await?;
        
        Ok(metadata_list)
    }

    /// Update user role in a chat
    pub async fn update_user_role(&self, user_id: &IdType, chat_id: &IdType, new_role: &UserRole) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE UserChatMetadata SET role = ? WHERE userId = ? AND chatId = ?",
            new_role as &UserRole,
            user_id,
            chat_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }

    /// Update messages received until timestamp
    pub async fn update_messages_received_until(
        &self, 
        user_id: &IdType, 
        chat_id: &IdType, 
        timestamp: &DateTime<Utc>
    ) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE UserChatMetadata SET lastDelivered = ? WHERE userId = ? AND chatId = ?",
            timestamp,
            user_id,
            chat_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }

    /// Check if user is member of chat
    pub async fn is_user_member(&self, user_id: &IdType, chat_id: &IdType) -> Result<bool, Error> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM UserChatMetadata WHERE userId = ? AND chatId = ?",
            user_id,
            chat_id
        )
        .fetch_one(&self.connection_pool)
        .await?;
        
        Ok(count.count > 0)
    }

    /// Check if user has admin or owner role in chat
    pub async fn is_user_admin_or_owner(&self, user_id: &IdType, chat_id: &IdType) -> Result<bool, Error> {
        let result = sqlx::query!(
            "SELECT role FROM UserChatMetadata WHERE userId = ? AND chatId = ?",
            user_id,
            chat_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        match result {
            Some(row) => {
                let role = row.role.unwrap_or_default();
                Ok(role == "admin" || role == "owner")
            },
            None => Ok(false)
        }
    }

    //MOD inutile??
    /// Get chat owner
    pub async fn get_chat_owner(&self, chat_id: &IdType) -> Result<Option<IdType>, Error> {
        let result = sqlx::query!(
            "SELECT userId FROM UserChatMetadata WHERE chatId = ? AND role = 'owner'",
            chat_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        Ok(result.map(|row| row.userId as IdType))
    }

    /// Remove user from chat (delete metadata entry)
    pub async fn remove_user_from_chat(&self, user_id: &IdType, chat_id: &IdType) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM UserChatMetadata WHERE userId = ? AND chatId = ?",
            user_id,
            chat_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }

    //MOD: inutile? si puo far tutto con 'update user role'
    /// Transfer ownership from one user to another in a chat
    pub async fn transfer_ownership(&self, from_user_id: &IdType, to_user_id: &IdType, chat_id: &IdType) -> Result<(), Error> {
        // Start a transaction for atomicity
        let mut tx = self.connection_pool.begin().await?;
        
        // Update the old owner to admin
        sqlx::query!(
            "UPDATE UserChatMetadata SET role = 'admin' WHERE userId = ? AND chatId = ?",
            from_user_id,
            chat_id
        )
        .execute(&mut *tx)
        .await?;
        
        // Update the new owner
        sqlx::query!(
            "UPDATE UserChatMetadata SET role = 'owner' WHERE userId = ? AND chatId = ?",
            to_user_id,
            chat_id
        )
        .execute(&mut *tx)
        .await?;
        
        // Commit the transaction
        tx.commit().await?;
        
        Ok(())
    }

    pub async fn find_all_by_user_id(&self, user_id: &IdType) -> Result<Vec<UserChatMetadata>, Error> {
        let result = sqlx::query_as!(
        UserChatMetadata,
        r#"
        SELECT
            userId as user_id,
            chatId as chat_id,
            role as "user_role: UserRole",
            member_since,
            messages_visible_from,
            messages_received_until
        FROM UserChatMetadata
        WHERE userId = ?
        "#,
        user_id
    )
            .fetch_all(&self.connection_pool)
            .await?;

        Ok(result)
    }

}


impl Crud<UserChatMetadata, IdType> for UserChatMetadataRepository {
    async fn create(&self, item: &UserChatMetadata) -> Result<UserChatMetadata, Error> {
        sqlx::query!(
            r#"
            INSERT INTO UserChatMetadata (userId, chatId, role, lastDelivered, deliverFrom) 
            VALUES (?, ?, ?, NULL, NULL)
            "#,
            item.user_id,
            item.chat_id,
            item.user_role as UserRole
        )
        .execute(&self.connection_pool)
        .await?;

        // Return the created metadata
        Ok(item.clone())
    }

    async fn read(&self, id: &IdType) -> Result<Option<UserChatMetadata>, Error> {
        // For UserChatMetadata, we'll interpret the ID as user_id for simplicity
        // In real scenarios, you might want a composite key approach
        let metadata = sqlx::query_as!(
            UserChatMetadata,
            r#"
            SELECT 
                userId as user_id,
                chatId as chat_id,
                role as "user_role: UserRole",
                DATETIME('now') as "member_since: DateTime<Utc>",
                DATETIME('1970-01-01') as "messages_visible_from: DateTime<Utc>",
                DATETIME('now') as "messages_received_until: DateTime<Utc>"
            FROM UserChatMetadata 
            WHERE userId = ?
            LIMIT 1
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        Ok(metadata)
    }

    async fn update(&self, item: &UserChatMetadata) -> Result<UserChatMetadata, Error> {
        sqlx::query!(
            r#"
            UPDATE UserChatMetadata 
            SET role = ?
            WHERE userId = ? AND chatId = ?
            "#,
            item.user_role as UserRole,
            item.user_id,
            item.chat_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        // Return the updated metadata
        Ok(item.clone())
    }

    async fn delete(&self, id: &IdType) -> Result<(), Error> {
        // Delete all metadata for a user (interpretation of the ID parameter)
        sqlx::query!(
            "DELETE FROM UserChatMetadata WHERE userId = ?",
            id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }
}

//INVITATION REPOSITORY
pub struct InvitationRepository {
    connection_pool: PoolType,
}

impl InvitationRepository {
    pub fn new(connection_pool: PoolType) -> Self {
        Self { connection_pool }
    }

    /// Get all pending invitations for a specific user
    pub async fn get_pending_invitations_for_user(&self, user_id: &IdType) -> Result<Vec<Invitation>, Error> {
        let invitations = sqlx::query_as!(
            Invitation,
            r#"
            SELECT 
                id as invite_id,
                groupId as chat_id,
                invitedUserId as invited_id,
                invitedById as invitee_id,
                status as "state: InvitationStatus"
            FROM Invitations 
            WHERE invitedUserId = ? AND status = 'pending'
            "#,
            user_id
        )
        .fetch_all(&self.connection_pool)
        .await?;
        
        Ok(invitations)
    }

    //MOD: controllo prima di inviare invito
    /// Check if there's already a pending invitation for user to chat
    pub async fn has_pending_invitation(&self, user_id: &IdType, chat_id: &IdType) -> Result<bool, Error> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM Invitations WHERE invitedUserId = ? AND groupId = ? AND status = 'pending'",
            user_id,
            chat_id
        )
        .fetch_one(&self.connection_pool)
        .await?;
        
        Ok(count.count > 0)
    }

    /// Update invitation status (accept/reject)
    pub async fn update_invitation_status(&self, invitation_id: &IdType, new_status: &InvitationStatus) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE Invitations SET status = ? WHERE id = ?",
            new_status as &InvitationStatus,
            invitation_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }
}

impl Crud<Invitation, IdType> for InvitationRepository {
    async fn create(&self, item: &Invitation) -> Result<Invitation, Error> {
        // Insert invitation using MySQL syntax
        let result = sqlx::query!(
            r#"
            INSERT INTO Invitations (groupId, invitedUserId, invitedById, status) 
            VALUES (?, ?, ?, ?)
            "#,
            item.chat_id,
            item.invited_id,
            item.invitee_id,
            item.state as InvitationStatus
        )
        .execute(&self.connection_pool)
        .await?;

        // Get the last inserted ID
        let new_id = result.last_insert_id() as IdType;

        // Return the created invitation with the new ID
        Ok(Invitation {
            invite_id: new_id,
            chat_id: item.chat_id,
            invited_id: item.invited_id,
            invitee_id: item.invitee_id,
            state: item.state.clone(),
        })
    }

    async fn read(&self, id: &IdType) -> Result<Option<Invitation>, Error> {
        let invitation = sqlx::query_as!(
            Invitation,
            r#"
            SELECT 
                id as invite_id,
                groupId as chat_id,
                invitedUserId as invited_id,
                invitedById as invitee_id,
                status as "state: InvitationStatus"
            FROM Invitations 
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        Ok(invitation)
    }

    async fn update(&self, item: &Invitation) -> Result<Invitation, Error> {
        sqlx::query!(
            r#"
            UPDATE Invitations 
            SET groupId = ?, invitedUserId = ?, invitedById = ?, status = ?
            WHERE id = ?
            "#,
            item.chat_id,
            item.invited_id,
            item.invitee_id,
            item.state as InvitationStatus,
            item.invite_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        // Return the updated invitation
        Ok(item.clone())
    }

    async fn delete(&self, id: &IdType) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM Invitations WHERE id = ?",
            id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }
}

// CHAT REPOSITORY
pub struct ChatRepository {
    connection_pool: PoolType,
}

impl ChatRepository {
    pub fn new(connection_pool: PoolType) -> Self {
        Self { connection_pool }
    }

    /// Get all chats where user is a member
    pub async fn get_chats_by_user(&self, user_id: &IdType) -> Result<Vec<Chat>, Error> {
        let chats = sqlx::query_as!(
            Chat,
            r#"
            SELECT 
                c.id as chat_id,
                c.title,
                c.description,
                c.type as "chat_type: ChatType"
            FROM Chats c
            INNER JOIN UserChatMetadata ucm ON c.id = ucm.chatId
            WHERE ucm.userId = ?
            "#,
            user_id
        )
        .fetch_all(&self.connection_pool)
        .await?;
        
        Ok(chats)
    }
//MOD opzione in piu per cercare chat private tra due utenti (possiamo evitare un ud come input)
    /// Get private chat between two users (if exists)
    pub async fn get_private_chat_between_users(&self, user1_id: &IdType, user2_id: &IdType) -> Result<Option<Chat>, Error> {
        let chat = sqlx::query_as!(
            Chat,
            r#"
            SELECT DISTINCT
                c.id as chat_id,
                c.title,
                c.description,
                c.type as "chat_type: ChatType"
            FROM Chats c
            INNER JOIN UserChatMetadata ucm1 ON c.id = ucm1.chatId
            INNER JOIN UserChatMetadata ucm2 ON c.id = ucm2.chatId
            WHERE c.type = 'Private' 
            AND ucm1.userId = ? 
            AND ucm2.userId = ?
            AND ucm1.userId != ucm2.userId
            "#,
            user1_id,
            user2_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        Ok(chat)
    }

    //MOD: assumo title univoco
    /// Get group chat by title 
    pub async fn get_groups_by_title(&self, title_group: &Option<String>) -> Result<Option<Chat>, Error> {
        let chats = sqlx::query_as!(
            Chat,
            r#"
            SELECT 
                id as chat_id,
                title,
                description,
                type as "chat_type: ChatType"
            FROM Chats 
            WHERE type = 'Group' and title = ?
            "#, 
            title_group
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        Ok(chats)
    }

    //MOD: forse utile per controlli
    /// Check if chat exists and is of specified type
    pub async fn is_chat_type(&self, chat_id: &IdType, expected_type: &ChatType) -> Result<bool, Error> {
        let result = sqlx::query!(
            "SELECT type FROM Chats WHERE id = ?",
            chat_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        match result {
            Some(row) => {
                let chat_type_str = row.r#type.unwrap_or_default();
                let matches = match expected_type {
                    ChatType::Group => chat_type_str == "Group",
                    ChatType::Private => chat_type_str == "Private",
                };
                Ok(matches)
            },
            None => Ok(false)
        }
    }

    /// Update chat title and description (for groups)
    pub async fn update_chat_description(&self, chat_id: &IdType, description: &Option<String>) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE Chats SET description = ? WHERE id = ?",
            description,
            chat_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }
}

impl Crud<Chat, IdType> for ChatRepository {
    async fn create(&self, item: &Chat) -> Result<Chat, Error> {
        // Insert chat using MySQL syntax
        let result = sqlx::query!(
            r#"
            INSERT INTO Chats (title, description, type) 
            VALUES (?, ?, ?)
            "#,
            item.title,
            item.description,
            item.chat_type as ChatType
        )
        .execute(&self.connection_pool)
        .await?;

        // Get the last inserted ID
        let new_id = result.last_insert_id() as IdType;

        // Return the created chat with the new ID
        Ok(Chat {
            chat_id: new_id,
            title: item.title.clone(),
            description: item.description.clone(),
            chat_type: item.chat_type.clone(),
        })
    }

    async fn read(&self, id: &IdType) -> Result<Option<Chat>, Error> {
        let chat = sqlx::query_as!(
            Chat,
            r#"
            SELECT 
                id as chat_id,
                title,
                description,
                type as "chat_type: ChatType"
            FROM Chats 
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        
        Ok(chat)
    }

    async fn update(&self, item: &Chat) -> Result<Chat, Error> {
        sqlx::query!(
            r#"
            UPDATE Chats 
            SET title = ?, description = ?, type = ?
            WHERE id = ?
            "#,
            item.title,
            item.description,
            item.chat_type as ChatType,
            item.chat_id
        )
        .execute(&self.connection_pool)
        .await?;
        
        // Return the updated chat
        Ok(item.clone())
    }

    async fn delete(&self, id: &IdType) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM Chats WHERE id = ?",
            id
        )
        .execute(&self.connection_pool)
        .await?;
        
        Ok(())
    }
}


//************************** UNIT TEST **************************//
//howto guide : https://docs.rs/sqlx/latest/sqlx/attr.test.html
/*
#[cfg(test)]
mod tests {
    use super::*;

    // Qui ho messo un esempio solo per mostrare come fare, notasi la sturttura in moduli e sottomoduli per chiarezza.

    mod user_repo {
        use super::*;

        /*
        Spiegazione:
        - Prima del test, viene creato un database isolato per eseguire il test senza influenzare il DB originale.
        - Lo schema viene costruito applicando i file SQL presenti nella cartella `migrations`, in ordine alfabetico.
        - Successivamente, le tabelle vengono inizializzate con dati definiti nei file SQL della cartella `fixtures`.
        - I file in fixtures sono quindi liste di insert into tabella values (...)
        - La selezione di quale pacchetto di entry caricare viene fatta scrivendo il nome del file dentro la macro scripts.
        - Non serve scrivere dentro scripts l'intera lista di files, ma solo quello che serve, altrimenti i test durano una vita.
        - Questo garantisce test isolati e ripetibili.
        - Se un test fallisce, il database isolato non viene eliminato, permettendo di analizzare l'errore in MySQL.
        */

        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../fixtures", scripts("popolate_users"))
        )]
        async fn test_create_user(pool: PoolType) -> sqlx::Result<()> {
            // effettuo l'azione
            sqlx::query_as!(
                User,
                "INSERT INTO users (id, name) VALUES (?, ?)",
                1,
                "Alice"
            )
            .execute(&pool)
            .await?;

            // verifico il risultato.
            let user = sqlx::query_as!(User, "SELECT id, name FROM users WHERE id = ?", 1)
                .fetch_one(&pool)
                .await?;

            assert_eq!(user.id, 1);
            assert_eq!(user.name, "Alice");

            Ok(())
        }
    }
}
*/
