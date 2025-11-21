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
      <div
        className={`d-flex ${styles.systemContainer}`}
      >
        <div
          className={`${styles.messageCard} ${styles.systemMessage}`}
          style={{ margin: 0 }} // rimuove margin ereditato
        >
          <span className={styles.messageContent}>{message.content}</span>
        </div>
      </div>
    );
  }


  return (
    <div className={`d-flex mb-1 ${isOwnMessage ? 'justify-content-end' : 'justify-content-start'}`}>
      <div className={`${styles.messageCard} ${isOwnMessage ? styles.ownMessage : styles.otherMessage}`}>
        {!isOwnMessage && senderUsername && (
          <div className={styles.senderName}>{senderUsername}</div>
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
