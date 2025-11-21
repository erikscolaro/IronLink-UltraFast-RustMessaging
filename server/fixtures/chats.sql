-- ============================================================
-- FIXTURE: Chats
-- ============================================================
-- Dipende da: users.sql
-- Chat di test con membri associati
-- ============================================================

-- Crea le chat
INSERT INTO chats (chat_id, title, description, chat_type) VALUES
(1, 'General Chat', 'Chat generale per tutti', 'GROUP'),
(2, 'Private Alice-Bob', NULL, 'PRIVATE'),
(3, 'Dev Team', 'Chat del team di sviluppo', 'GROUP');

-- Associa utenti alle chat con metadata
-- Colonne richieste: user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since
INSERT INTO userchatmetadata (user_id, chat_id, messages_visible_from, messages_received_until, user_role, member_since) VALUES
-- General Chat: alice (OWNER), bob (MEMBER), charlie (MEMBER)
(1, 1, NOW(), NOW(), 'OWNER', NOW()),
(2, 1, NOW(), NOW(), 'MEMBER', NOW()),
(3, 1, NOW(), NOW(), 'MEMBER', NOW()),

-- Private Alice-Bob: alice (OWNER), bob (MEMBER)
(1, 2, NOW(), NOW(), 'OWNER', NOW()),
(2, 2, NOW(), NOW(), 'MEMBER', NOW()),

-- Dev Team: alice (OWNER), charlie (ADMIN)
(1, 3, NOW(), NOW(), 'OWNER', NOW()),
(3, 3, NOW(), NOW(), 'ADMIN', NOW());
