// Home Page - Pagina principale con chat
import { useEffect, useState } from 'react';
import { ChatDTO } from '../../models/types';
import { Container, Row, Col, Spinner } from 'react-bootstrap';
import * as api from '../../services/api';
import { useWebSocket } from '../../context/WebSocketContext';
import Sidebar from '../../components/Sidebar/Sidebar';
import ChatArea from '../../components/ChatArea/ChatArea';
import ChatInfo from '../../components/ChatInfo/ChatInfo';
import ProfileModal from '../../components/ProfileModal/ProfileModal';
import styles from './Home.module.css';

export default function Home() {
  const { onChatAdded, onChatRemoved, subscribeToChat } = useWebSocket();
  const [chats, setChats] = useState<ChatDTO[]>([]);
  const [selectedChatId, setSelectedChatId] = useState<number | null>(null);
  const [showChatInfo, setShowChatInfo] = useState(false);
  const [showProfile, setShowProfile] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [cleanChatTrigger, setCleanChatTrigger] = useState(0); // Trigger per pulire messaggi
  const [chatsWithUnread, setChatsWithUnread] = useState<Set<number>>(new Set());
  const [inviteMode, setInviteMode] = useState<{
    chatId: number;
    existingMemberIds: number[];
    onInvite: (userId: number) => Promise<void>;
    onCancel: () => void;
  } | null>(null);

  // Carica le chat all'avvio
  useEffect(() => {
    loadChats();
  }, []);

  // Ascolta messaggi su tutte le chat per aggiornare il pallino non letto
  useEffect(() => {
    const unsubscribes: (() => void)[] = [];

    // Sottoscrivi a tutte le chat
    chats.forEach(chat => {
      const unsubscribe = subscribeToChat(chat.chat_id, (message) => {
        // Se il messaggio arriva su una chat diversa da quella selezionata, marca come non letta
        if (message.chat_id && message.chat_id !== selectedChatId) {
          setChatsWithUnread(prev => new Set(prev).add(message.chat_id!));
        }
      });
      unsubscribes.push(unsubscribe);
    });

    return () => {
      unsubscribes.forEach(unsub => unsub());
    };
  }, [chats, selectedChatId, subscribeToChat]);

  // Gestisci AddChat dal WebSocket
  useEffect(() => {
    const unsubscribe = onChatAdded(async (_chatId) => {
      try {
        // Ricarica la lista completa per avere i dettagli della nuova chat
        const chatsData = await api.listChats();
        setChats(chatsData);
      } catch (error) {
        console.error('Errore ricaricamento chat dopo AddChat:', error);
      }
    });

    return unsubscribe;
  }, [onChatAdded]);

  // Gestisci RemoveChat dal WebSocket
  useEffect(() => {
    const unsubscribe = onChatRemoved((chatId) => {
      setChats(prev => prev.filter(chat => chat.chat_id !== chatId));
      // Se era selezionata, deseleziona
      if (selectedChatId === chatId) {
        setSelectedChatId(null);
      }
    });

    return unsubscribe;
  }, [onChatRemoved, selectedChatId]);

  const loadChats = async () => {
    setIsLoading(true);
    try {
      const chatsData = await api.listChats();
      setChats(chatsData);
    } catch (error) {
      console.error('Errore caricamento chat:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const selectedChat = chats.find((chat) => chat.chat_id === selectedChatId);

  if (isLoading) {
    return (
      <Container fluid className="d-flex align-items-center justify-content-center vh-100">
        <div className="text-center">
          <Spinner animation="border" variant="secondary" />
          <p className="mt-2">Caricamento...</p>
        </div>
      </Container>
    );
  }

  return (
    <Container fluid className={styles.homeContainer}>
      <Row className="h-100 g-3">
        {/* Sidebar sinistra - nascosta su mobile quando c'è una chat selezionata */}
        <Col
          xs={12}
          md={4}
          className={`h-100 ${selectedChat ? 'd-none d-md-block' : ''}`}
        >
          <Sidebar
            chats={chats}
            selectedChatId={selectedChatId}
            onSelectChat={(chatId) => {
              setSelectedChatId(chatId);
              setShowChatInfo(false);
              // Rimuovi il pallino quando apri la chat
              setChatsWithUnread(prev => {
                const newSet = new Set(prev);
                newSet.delete(chatId);
                return newSet;
              });
            }}
            onShowProfile={() => setShowProfile(true)}
            onRefreshChats={loadChats}
            inviteMode={inviteMode}
            chatsWithUnread={chatsWithUnread}
          />
        </Col>

        {/* Area centrale della chat */}
        <Col
          xs={12}
          md={showChatInfo ? 4 : 8}
          className={`h-100 ${!selectedChat ? 'd-none d-md-block' : ''}`}
        >
          {selectedChat ? (
            <ChatArea
              chat={selectedChat}
              onShowInfo={() => setShowChatInfo(!showChatInfo)}
              onBack={() => setSelectedChatId(null)}
              cleanChatTrigger={cleanChatTrigger}
            />
          ) : (
            <div className="d-none d-md-flex flex-column align-items-center justify-content-center h-100">
              <i className="bi bi-chat-dots mb-3" style={{ fontSize: '4rem' }}></i>
              <h2>Seleziona una chat per iniziare</h2>
              <p>Scegli una chat dalla lista a sinistra per vedere i messaggi</p>
            </div>
          )}
        </Col>

        {/* Pannello info chat (se visibile) - hidden su mobile */}
        {selectedChat && showChatInfo && (
          <Col xs={12} md={4} className="h-100 d-none d-md-block">
            <ChatInfo
              chat={selectedChat}
              isVisible={showChatInfo}
              onClose={() => setShowChatInfo(false)}
              onStartInvite={(chatId, memberIds, onInvite) => {
                setInviteMode({
                  chatId,
                  existingMemberIds: memberIds,
                  onInvite,
                  onCancel: () => setInviteMode(null)
                });
              }}
              onChatLeft={() => {
                // Rimuovi la chat dalla lista e deseleziona
                setChats(prevChats => prevChats.filter(c => c.chat_id !== selectedChat.chat_id));
                setSelectedChatId(null);
                setShowChatInfo(false);
              }}
              onChatCleaned={() => {
                // Incrementa il trigger per segnalare a ChatArea di pulire i messaggi
                setCleanChatTrigger(prev => prev + 1);
              }}
            />
          </Col>
        )}
      </Row>

      {/* Modale profilo */}
      <ProfileModal
        show={showProfile}
        onHide={() => setShowProfile(false)}
      />
    </Container>
  );
}
