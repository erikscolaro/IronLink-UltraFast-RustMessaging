//! Repositories module - Coordinatore per tutti i repository del progetto
//!
//! Questo modulo organizza i repository in sotto-moduli separati per una migliore manutenibilità.
//! Ogni repository gestisce le operazioni di database per una specifica entità.

// ************************* NOTA IMPORTANTE SU SQLX ************************* //

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

// ************************* MODULI REPOSITORY ************************* //

// Dichiarazione dei sotto-moduli
pub mod chat;
pub mod invitation;
pub mod message;
pub mod traits;
pub mod user;
pub mod user_chat_metadata;

// Re-esportazione dei trait per facilitare l'import
pub use traits::{Create, Delete, Read, Update};

// Note: ReadMany is exported but not yet used. It will be available when needed.

// Re-esportazione delle struct dei repository per facilitare l'import
pub use chat::ChatRepository;
pub use invitation::InvitationRepository;
pub use message::MessageRepository;
pub use user::UserRepository;
pub use user_chat_metadata::UserChatMetadataRepository;
