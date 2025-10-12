# Testing Guide - Ruggine Chat

Questa guida spiega come configurare ed eseguire i test per Ruggine Chat.

## 📋 Indice

1. [Architettura dei Test](#architettura-dei-test)
2. [Configurazione](#configurazione)
3. [Fixtures](#fixtures)
4. [Esecuzione dei Test](#esecuzione-dei-test)
5. [Scrivere Nuovi Test](#scrivere-nuovi-test)

---

## 🏗️ Architettura dei Test

Il progetto usa un'architettura di test **ibrida**:

### **1. Integration Tests** (`tests/`)
Test end-to-end per gli endpoint HTTP:
- `tests/api_auth.rs` - Autenticazione (login, register)
- `tests/api_users.rs` - Gestione utenti
- `tests/api_chats.rs` - Gestione chat e messaggi

### **2. Unit Tests** (inline nei repository)
Test delle funzioni CRUD dei repository:
- `src/repositories/user.rs`
- `src/repositories/message.rs`
- `src/repositories/chat.rs`
- `src/repositories/user_chat_metadata.rs`
- `src/repositories/invitation.rs`

### **3. Test Helpers** (`tests/common/`)
Funzioni di utilità condivise:
- `create_test_jwt()` - Genera token JWT per i test

---

## ⚙️ Configurazione

### Prerequisiti

1. **MySQL Server** in esecuzione
2. **Database rugginedb** creato (esegui `migrations/1_create_database.sql`)
3. **Utente root** con password configurata

### Configurazione DATABASE_URL

I test usano il macro `#[sqlx::test]` che richiede un utente con **privilegi di superuser** (root) per:
- Creare database di test temporanei
- Applicare automaticamente le migrations
- Distruggere i database al termine dei test

**⚠️ IMPORTANTE**: Durante i test, ogni test riceve un database **isolato** (es: `_sqlx_test_1234`), quindi non c'è rischio di conflitti.

#### Metodo 1: Variabile d'Ambiente Temporanea

```powershell
# Windows PowerShell
$env:DATABASE_URL="mysql://root:your_root_password@127.0.0.1:3306"
cargo test

# Linux/macOS
DATABASE_URL=mysql://root:your_root_password@127.0.0.1:3306 cargo test
```

#### Metodo 2: File `.env.test` (Raccomandato)

Crea un file `.env.test` nella root di `server/`:

```properties
DATABASE_URL=mysql://root:your_root_password@127.0.0.1:3306
```

Poi esegui:

```bash
cargo test
```

**Nota**: SQLx legge automaticamente `.env.test` se esiste, altrimenti fallback su `.env`.

---

## 📦 Fixtures

I fixtures sono file SQL con dati di test predefiniti, situati in `fixtures/`:

### Fixtures Disponibili

#### `fixtures/users.sql`
Crea 3 utenti di test:
- **alice** (id=1, password: `password123`)
- **bob** (id=2, password: `password123`)
- **charlie** (id=3, password: `password123`)

#### `fixtures/chats.sql`
Crea 3 chat e associazioni utente-chat:
- **General Chat** (id=1) - alice, bob, charlie
- **Private Alice-Bob** (id=2) - alice, bob
- **Dev Team** (id=3) - alice, charlie

**Dipendenze**: `users.sql`

#### `fixtures/messages.sql`
Crea 7 messaggi di test nelle varie chat.

**Dipendenze**: `users.sql`, `chats.sql`

### Come Usare i Fixtures

Specifica i fixtures nel macro `#[sqlx::test]`:

```rust
// Test con un singolo fixture
#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_search_users(pool: MySqlPool) -> sqlx::Result<()> {
    // Alice, Bob e Charlie sono già nel database
    // ...
}

// Test con fixtures multipli (applicati in ordine)
#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "chats", "messages")))]
async fn test_get_messages(pool: MySqlPool) -> sqlx::Result<()> {
    // Database popolato con utenti, chat e messaggi
    // ...
}

// Test senza fixtures (database vuoto)
#[sqlx::test]
async fn test_create_user(pool: MySqlPool) -> sqlx::Result<()> {
    // Database vuoto, solo schema
    // ...
}
```

**Nota sui Path**:
- Integration tests (`tests/`): usa `path = "../fixtures"`
- Unit tests (`src/repositories/`): usa `path = "../../fixtures"`

---

## 🚀 Esecuzione dei Test

### Esegui Tutti i Test

```bash
# Assicurati che DATABASE_URL punti a root
$env:DATABASE_URL="mysql://root:password@127.0.0.1:3306"
cargo test
```

### Esegui Test Specifici

```bash
# Solo integration tests
cargo test --test api_auth

# Solo unit tests di un repository
cargo test user::tests

# Test specifico per nome
cargo test test_login_with_existing_user

# Con output verbose
cargo test -- --nocapture
```

### Debug dei Test

```bash
# Mostra output di println! anche per test passati
cargo test -- --nocapture --test-threads=1

# Esegui un singolo test
cargo test test_login_with_existing_user -- --exact
```

---

## ✍️ Scrivere Nuovi Test

### Integration Test (Endpoint HTTP)

```rust
// tests/api_auth.rs

#[sqlx::test(fixtures(path = "../fixtures", scripts("users")))]
async fn test_new_endpoint(pool: MySqlPool) -> sqlx::Result<()> {
    // 1. Setup: crea AppState, request, ecc.
    let app_state = AppState { pool: pool.clone(), config: /* ... */ };
    
    // 2. Action: esegui la richiesta HTTP
    let response = /* ... */;
    
    // 3. Assert: verifica response e stato del database
    assert_eq!(response.status(), StatusCode::OK);
    
    Ok(())
}
```

### Unit Test (Repository)

```rust
// src/repositories/user.rs

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::MySqlPool;

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("users")))]
    async fn test_new_repository_method(pool: MySqlPool) -> sqlx::Result<()> {
        let repo = UserRepository::new(pool);
        
        // Test della funzione del repository
        let result = repo.some_method().await?;
        
        assert!(result.is_ok());
        Ok(())
    }
}
```

### Best Practices

1. **Usa fixtures quando possibile** invece di creare dati manualmente
2. **Ogni test deve essere indipendente** - non fare affidamento sullo stato di altri test
3. **Usa nomi descrittivi** - `test_login_with_invalid_password` è meglio di `test_login_fail`
4. **Testa edge cases** - input vuoti, valori limite, errori
5. **Ritorna `sqlx::Result<()>`** per gestire errori del database
6. **Documenta i test complessi** con commenti

---

## 🔧 Troubleshooting

### "couldn't read fixtures/users.sql"

**Causa**: Path relativo errato.

**Soluzione**:
- Integration tests: `path = "../fixtures"`
- Unit tests (repositories): `path = "../../fixtures"`

### "Access denied for user 'ruggine'"

**Causa**: Stai usando l'utente applicativo invece di root.

**Soluzione**: Cambia `DATABASE_URL` a root prima di eseguire i test:
```bash
$env:DATABASE_URL="mysql://root:password@127.0.0.1:3306"
```

### "database _sqlx_test_xxx already exists"

**Causa**: Test precedente crashato senza pulizia.

**Soluzione**: Elimina manualmente i database di test:
```sql
DROP DATABASE IF EXISTS _sqlx_test_<numero>;
```

O esegui:
```bash
cargo clean
cargo test
```

### I test sono lenti

**Causa**: Ogni test crea un nuovo database.

**Soluzione**:
- Usa `#[sqlx::test]` solo dove necessario
- Raggruppa test simili in un unico test con sotto-casi
- Esegui test in parallelo (default): `cargo test`

---

## 📚 Risorse

- [SQLx Testing Documentation](https://docs.rs/sqlx/latest/sqlx/attr.test.html)
- [Tokio Testing](https://tokio.rs/tokio/topics/testing)
- [Rust Testing Best Practices](https://doc.rust-lang.org/book/ch11-00-testing.html)

---

**Ultimo aggiornamento**: Ottobre 2025
