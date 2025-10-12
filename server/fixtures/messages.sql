-- ============================================================
-- FIXTURE: Messages
-- ============================================================
-- Dipende da: users.sql, chats.sql
-- Messaggi di test per le chat
-- ============================================================

INSERT INTO messages (message_id, chat_id, sender_id, content, message_type, created_at) VALUES
-- General Chat (chat_id=1)
(1, 1, 1, 'Hello everyone!', 'USERMESSAGE', NOW() - INTERVAL 10 MINUTE),
(2, 1, 2, 'Hi Alice!', 'USERMESSAGE', NOW() - INTERVAL 9 MINUTE),
(3, 1, 3, 'Good morning!', 'USERMESSAGE', NOW() - INTERVAL 8 MINUTE),

-- Private Alice-Bob (chat_id=2)
(4, 2, 1, 'Hey Bob, are you there?', 'USERMESSAGE', NOW() - INTERVAL 5 MINUTE),
(5, 2, 2, 'Yes, what''s up?', 'USERMESSAGE', NOW() - INTERVAL 4 MINUTE),

-- Dev Team (chat_id=3)
(6, 3, 1, 'Let''s start the meeting', 'USERMESSAGE', NOW() - INTERVAL 2 MINUTE),
(7, 3, 3, 'I''m ready!', 'USERMESSAGE', NOW() - INTERVAL 1 MINUTE);
