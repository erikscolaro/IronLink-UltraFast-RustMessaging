use crate::entities::{Chat, IdType, Invitation, Message, User, UserChatMetadata};
use sqlx::{Error, MySqlPool};

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
//TODO: "bisogna aggiungere alle definizioni dei models i tipi che vengono usati nel database, chiarire questo aspetto"

// USER REPO
pub struct UserRepository {
    connection_pool: PoolType,
}

impl UserRepository {
    pub fn new(connection_pool: PoolType) -> UserRepository {
        Self { connection_pool }
    }

    // ricerca per username esatto, per ricerca globale username parziale usare altro metodo
    pub async fn find_by_username(&self, username: &String) -> Result<Option<User>, Error> {
        todo!()
    }
}

impl Crud<User, IdType> for UserRepository {
    async fn create(&self, item: &User) -> Result<User, Error> {
        todo!()
    }

    async fn read(&self, id: &IdType) -> Result<Option<User>, Error> {
        todo!()
    }

    async fn update(&self, item: &User) -> Result<User, Error> {
        todo!()
    }

    async fn delete(&self, id: &IdType) -> Result<(), Error> {
        todo!()
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
}

impl Crud<Message, IdType> for MessageRepository {
    async fn create(&self, item: &Message) -> Result<Message, Error> {
        todo!()
    }

    async fn read(&self, id: &IdType) -> Result<Option<Message>, Error> {
        todo!()
    }

    async fn update(&self, item: &Message) -> Result<Message, Error> {
        todo!()
    }

    async fn delete(&self, id: &IdType) -> Result<(), Error> {
        todo!()
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
}

impl Crud<UserChatMetadata, IdType> for UserChatMetadataRepository {
    async fn create(&self, item: &UserChatMetadata) -> Result<UserChatMetadata, Error> {
        todo!()
    }

    async fn read(&self, id: &IdType) -> Result<Option<UserChatMetadata>, Error> {
        todo!()
    }

    async fn update(&self, item: &UserChatMetadata) -> Result<UserChatMetadata, Error> {
        todo!()
    }

    async fn delete(&self, id: &IdType) -> Result<(), Error> {
        todo!()
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
}

impl Crud<Invitation, IdType> for InvitationRepository {
    async fn create(&self, item: &Invitation) -> Result<Invitation, Error> {
        todo!()
    }

    async fn read(&self, id: &IdType) -> Result<Option<Invitation>, Error> {
        todo!()
    }

    async fn update(&self, item: &Invitation) -> Result<Invitation, Error> {
        todo!()
    }

    async fn delete(&self, id: &IdType) -> Result<(), Error> {
        todo!()
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
}

impl Crud<Chat, IdType> for ChatRepository {
    async fn create(&self, item: &Chat) -> Result<Chat, Error> {
        todo!()
    }

    async fn read(&self, id: &IdType) -> Result<Option<Chat>, Error> {
        todo!()
    }

    async fn update(&self, item: &Chat) -> Result<Chat, Error> {
        todo!()
    }

    async fn delete(&self, id: &IdType) -> Result<(), Error> {
        todo!()
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
