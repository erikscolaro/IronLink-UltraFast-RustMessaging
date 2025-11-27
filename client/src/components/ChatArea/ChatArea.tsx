// ChatArea - Area principale della chat con messaggi
import { useEffect, useState, useRef, useCallback } from 'react';
import { ChatDTO, MessageDTO, MessageType, getUserId } from '../../models/types';
import { Spinner } from 'react-bootstrap';
import { useAuth } from '../../context/AuthContext';
import { useWebSocket } from '../../context/WebSocketContext';
import * as api from '../../services/api';
import ChatHeader from '../ChatHeader/ChatHeader';
import ChatMessage from '../ChatMessage/ChatMessage';
import ChatInput from '../ChatInput/ChatInput';
import styles from './ChatArea.module.css';

interface ChatAreaProps {
  chat: ChatDTO;
  onShowInfo: () => void;
  onBack?: () => void; // Per tornare alla lista chat su mobile
  cleanChatTrigger?: number; // Trigger per pulire i messaggi
}

export default function ChatArea({ chat, onShowInfo, onBack, cleanChatTrigger }: ChatAreaProps) {
  const { user } = useAuth();
  const { sendMessage, subscribeToChat } = useWebSocket();
  const [messages, setMessages] = useState<MessageDTO[]>([]);
  const [members, setMembers] = useState<Map<number, string>>(new Map());
  const [isLoading, setIsLoading] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const messagesContainerRef = useRef<HTMLDivElement>(null);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  // Carica altri messaggi quando si scrolla in cima
  const handleScroll = useCallback(async () => {
    if (isLoadingMore || isLoading) return;
    const container = messagesContainerRef.current;
    if (!container) return;
    // Funziona solo se la divisione è scrollabile
    if (container.scrollHeight <= container.clientHeight) return;
    if (container.scrollTop === 0 && messages.length > 0) {
      setIsLoadingMore(true);
      const oldestMsg = messages[0];
      try {
        const moreMsgs = await api.getChatMessages(chat.chat_id, oldestMsg.created_at);
        if (moreMsgs.length > 0) {
          setMessages(prev => {
            // Unisci, rimuovi duplicati per message_id, ordina dal più vecchio al più recente
            const allMsgs = [...moreMsgs, ...prev];
            const seen = new Set();
            const unique = allMsgs.filter(msg => {
              if (msg.message_id && seen.has(msg.message_id)) return false;
              if (msg.message_id) seen.add(msg.message_id);
              return true;
            });
            unique.sort((a, b) => new Date(a.created_at ?? '').getTime() - new Date(b.created_at ?? '').getTime());
            return unique;
          });
        }
      } catch (e) {
        // opzionale: mostra errore
      } finally {
        setIsLoadingMore(false);
      }
    }
  }, [chat.chat_id, messages, isLoadingMore, isLoading]);

  // Carica messaggi e membri all'apertura della chat
  useEffect(() => {
    const loadChatData = async () => {
      setIsLoading(true);
      try {
        // Carica messaggi (arrivano in ordine DESC, invertiamo per avere i più recenti in basso)
        const msgs = await api.getChatMessages(chat.chat_id);
        setMessages(msgs.reverse());

        // Carica membri per ottenere gli username
        const chatMembers = await api.listChatMembers(chat.chat_id);
        const memberMap = new Map<number, string>();
        for (const member of chatMembers) {
          try {
            const userData = await api.getUserById(member.user_id);
            const userId = userData.id || userData.user_id;
            const username = userData.username;
            if (userId && username) {
              memberMap.set(userId, username);
            }
          } catch (error) {
            console.error(`Errore caricamento utente ${member.user_id}:`, error);
          }
        }
        setMembers(memberMap);

        // Porta la chat a scrollBottom dopo il caricamento
        // Forza scrollBottom dopo caricamento messaggi
        requestAnimationFrame(() => {
          const container = messagesContainerRef.current;
          if (container) {
            container.scrollTop = container.scrollHeight;
          }
        });
      } catch (error) {
        console.error('Errore caricamento dati chat:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadChatData();
  }, [chat.chat_id]);

  // Sottoscrivi ai messaggi WebSocket
  useEffect(() => {
    const unsubscribe = subscribeToChat(chat.chat_id, (newMessage) => {
      // Evita duplicati: controlla se il messaggio esiste già
      setMessages((prev) => {
        // Se ha message_id, controlla per ID
        if (newMessage.message_id) {
          const exists = prev.some(msg => msg.message_id === newMessage.message_id);
          if (exists) return prev;
        } else {
          // Senza message_id, controlla per contenuto+timestamp+sender
          const exists = prev.some(msg => 
            msg.content === newMessage.content &&
            msg.created_at === newMessage.created_at &&
            msg.sender_id === newMessage.sender_id
          );
          if (exists) return prev;
        }
        
        return [...prev, newMessage];
      });
    });

    return () => {
      unsubscribe();
    };
  }, [chat.chat_id, subscribeToChat]);

  // ScrollBottom solo al primo caricamento della chat
  const firstLoad = useRef(true);
  useEffect(() => {
    if (firstLoad.current && messages.length > 0) {
      requestAnimationFrame(() => {
        const container = messagesContainerRef.current;
        if (container) {
          container.scrollTop = container.scrollHeight;
        }
      });
      firstLoad.current = false;
    }
  }, [messages, chat.chat_id]);

  useEffect(() => {
    firstLoad.current = true;
  }, [chat.chat_id]);

  // Aggiungi event listener per lo scroll
  useEffect(() => {
    const container = messagesContainerRef.current;
    if (!container) return;
    container.addEventListener('scroll', handleScroll);
    return () => {
      container.removeEventListener('scroll', handleScroll);
    };
  }, [handleScroll]);

  // Gestione pulizia chat tramite trigger
  useEffect(() => {
    if (cleanChatTrigger && cleanChatTrigger > 0) {
      setMessages([]);
    }
  }, [cleanChatTrigger]);

  const handleSendMessage = (content: string) => {
    if (!user) return;

    const userId = getUserId(user);
    if (!userId) {
      console.error('User ID non disponibile');
      return;
    }

    const newMessage: MessageDTO = {
      chat_id: chat.chat_id,
      sender_id: userId,
      content,
      message_type: MessageType.UserMessage,
      created_at: new Date().toISOString(),
    };

    sendMessage(newMessage);
  };

  // Ottiene l'username dell'altro utente in chat private
  const getOtherUsername = (): string | undefined => {
    if (!user) return undefined;
    const currentUserId = getUserId(user);
    for (const [userId, username] of members.entries()) {
      if (userId !== currentUserId) {
        return username;
      }
    }
    return undefined;
  };

  if (isLoading) {
    return (
      <div className="d-flex align-items-center justify-content-center h-100">
        <div className="text-center">
          <Spinner animation="border" variant="secondary" />
          <p className="mt-2">Caricamento chat...</p>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.chatArea}>
      <ChatHeader
        chat={chat}
        otherUsername={getOtherUsername()}
        onShowInfo={onShowInfo}
        onBack={onBack}
      />
      <div
        className={styles.messagesContainer}
        ref={messagesContainerRef}
      >
        {isLoadingMore && (
          <div className="d-flex align-items-center justify-content-center text-muted" style={{ minHeight: 30 }}>
            <Spinner animation="border" size="sm" /> Caricamento...
          </div>
        )}
        {messages.length === 0 ? (
          <div className="d-flex align-items-center justify-content-center h-100 text-muted">
            <p>Nessun messaggio ancora. Inizia la conversazione!</p>
          </div>
        ) : (
          messages.map((msg, index) => {
            const isOwnMessage = user ? msg.sender_id === getUserId(user) : false;
            const isPrivateChat = chat.chat_type === 'Private';
            // Determina se mostrare il nome utente:
            // 1. Mai nelle chat private
            // 2. Mai per i propri messaggi
            // 3. Mai se il messaggio precedente è dello stesso utente
            const previousMsg = index > 0 ? messages[index - 1] : null;
            const shouldShowUsername = !isPrivateChat && 
                                      !isOwnMessage && 
                                      (!previousMsg || previousMsg.sender_id !== msg.sender_id);
            return (
              <ChatMessage
                key={msg.message_id || `msg-${index}`}
                message={msg}
                senderUsername={shouldShowUsername ? (msg.sender_id ? members.get(msg.sender_id) : undefined) : undefined}
                isOwnMessage={isOwnMessage}
              />
            );
          })
        )}
        <div ref={messagesEndRef} />
      </div>
      <ChatInput onSendMessage={handleSendMessage} />
    </div>
  );
}
