// ChatArea - Area principale della chat con messaggi
import { useEffect, useState, useRef } from 'react';
import { ChatDTO, MessageDTO, MessageType } from '../../models/types';
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
}

export default function ChatArea({ chat, onShowInfo, onBack }: ChatAreaProps) {
  const { user } = useAuth();
  const { sendMessage, subscribeToChat } = useWebSocket();
  const [messages, setMessages] = useState<MessageDTO[]>([]);
  const [members, setMembers] = useState<Map<number, string>>(new Map());
  const [isLoading, setIsLoading] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Carica messaggi e membri all'apertura della chat
  useEffect(() => {
    const loadChatData = async () => {
      setIsLoading(true);
      try {
        // Carica messaggi
        const msgs = await api.getChatMessages(chat.chat_id);
        setMessages(msgs);

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
      setMessages((prev) => [...prev, newMessage]);
    });

    return unsubscribe;
  }, [chat.chat_id, subscribeToChat]);

  // Auto-scroll all'ultimo messaggio
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSendMessage = (content: string) => {
    if (!user) return;

    const newMessage: MessageDTO = {
      chat_id: chat.chat_id,
      sender_id: user.user_id,
      content,
      message_type: MessageType.UserMessage,
      created_at: new Date().toISOString(),
    };

    sendMessage(newMessage);
  };

  // Ottiene l'username dell'altro utente in chat private
  const getOtherUsername = (): string | undefined => {
    if (!user) return undefined;
    for (const [userId, username] of members.entries()) {
      if (userId !== user.user_id) {
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
      
      <div className={styles.messagesContainer}>
        {messages.length === 0 ? (
          <div className="d-flex align-items-center justify-content-center h-100 text-muted">
            <p>Nessun messaggio ancora. Inizia la conversazione!</p>
          </div>
        ) : (
          messages.map((msg, index) => (
            <ChatMessage
              key={msg.message_id || `msg-${index}`}
              message={msg}
              senderUsername={msg.sender_id ? members.get(msg.sender_id) : undefined}
              isOwnMessage={msg.sender_id === user?.user_id}
            />
          ))
        )}
        <div ref={messagesEndRef} />
      </div>
      
      <ChatInput onSendMessage={handleSendMessage} />
    </div>
  );
}
