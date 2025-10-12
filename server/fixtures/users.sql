-- ============================================================
-- FIXTURE: Users
-- ============================================================
-- Utenti di test per tutti i test
-- Password per tutti: "password123" 
-- Hash bcrypt: $2b$04$rN8qH.H8L2xGGXzXWdF5yOqKvH9P.8vZH7.7rKQJ0N8qH.H8L2xGG
-- (cost=4 per velocità nei test)
-- ============================================================

INSERT INTO users (user_id, username, password) VALUES
(1, 'alice', '$2b$04$rN8qH.H8L2xGGXzXWdF5yOqKvH9P.8vZH7.7rKQJ0N8qH.H8L2xGG'),
(2, 'bob', '$2b$04$rN8qH.H8L2xGGXzXWdF5yOqKvH9P.8vZH7.7rKQJ0N8qH.H8L2xGG'),
(3, 'charlie', '$2b$04$rN8qH.H8L2xGGXzXWdF5yOqKvH9P.8vZH7.7rKQJ0N8qH.H8L2xGG');
