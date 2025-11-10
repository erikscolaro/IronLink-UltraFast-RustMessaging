// ChatMessage - Componente per visualizzare un singolo messaggio
import { MessageDTO, MessageType } from '../../models/types';
import { Badge } from 'react-bootstrap';
import styles from './ChatMessage.module.css';

interface ChatMessageProps {
  message: MessageDTO;
  senderUsername?: string;
  isOwnMessage: boolean;
  status?: 'pending' | 'sent' | 'error';
}

export default function ChatMessage({ message, senderUsername, isOwnMessage, status = 'sent' }: ChatMessageProps) {
  // Formato ora da timestamp
  const formatTime = (timestamp: string | undefined) => {
    if (!timestamp) return '';
    const date = new Date(timestamp);
    return date.toLocaleTimeString('it-IT', { hour: '2-digit', minute: '2-digit' });
  };

  const isSystemMessage = message.message_type === MessageType.SystemMessage;

  if (isSystemMessage) {
    return (
      <div className="d-flex justify-content-center my-3">
        <Badge bg="secondary" className="py-2 px-3">
          {message.content}
        </Badge>
      </div>
    );
  }

  return (
    <div className={`d-flex mb-3 px-3 ${isOwnMessage ? 'justify-content-end' : 'justify-content-start'}`}>
      <div className={`${styles.messageCard} ${isOwnMessage ? styles.ownMessage : styles.otherMessage}`}>
        {!isOwnMessage && (
          <div className={styles.senderName}>{senderUsername || 'Utente'}</div>
        )}
        <div className={styles.messageContent}>{message.content}</div>
        <div className={styles.messageFooter}>
          <small className={styles.messageTime}>{formatTime(message.created_at)}</small>
          {isOwnMessage && (
            <span className={styles.messageStatus}>
              {status === 'pending' && <i className="bi bi-clock"></i>}
              {status === 'sent' && <i className="bi bi-check2"></i>}
              {status === 'error' && <i className="bi bi-exclamation-circle"></i>}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
