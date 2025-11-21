-- ============================================================
-- FIXTURE: Invitations
-- ============================================================
-- Dipende da: users.sql, chats.sql
-- Inviti di test per le chat
-- ============================================================

INSERT INTO invitations (invite_id, target_chat_id, invited_id, invitee_id, state, created_at) VALUES
-- Bob invita Charlie al General Chat (PENDING)
(1, 1, 3, 2, 'PENDING', NOW() - INTERVAL 1 HOUR),

-- Alice invita Bob al Dev Team (ACCEPTED)
(2, 3, 2, 1, 'ACCEPTED', NOW() - INTERVAL 2 HOUR),

-- Charlie invita Alice a un'altra chat (REJECTED)
(3, 1, 1, 3, 'REJECTED', NOW() - INTERVAL 3 HOUR);
