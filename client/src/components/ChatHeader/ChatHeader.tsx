// ChatHeader - Header della chat con nome e azioni
import { ChatDTO, ChatType } from '../../models/types';
import { Button } from 'react-bootstrap';
import styles from './ChatHeader.module.css';

interface ChatHeaderProps {
  chat: ChatDTO;
  otherUsername?: string; // Per chat private
  onShowInfo: () => void;
  onBack?: () => void; // Per tornare alla lista chat su mobile
}

export default function ChatHeader({ chat, otherUsername, onShowInfo, onBack }: ChatHeaderProps) {
  const isPrivate = chat.chat_type === ChatType.Private;
  const chatName = isPrivate ? (otherUsername || 'Chat Privata') : (chat.title || 'Chat di Gruppo');

  return (
    <div className={styles.chatHeader}>
      <div className={styles.chatInfo}>
        {/* Pulsante Back per mobile */}
        {onBack && (
          <Button
            variant='primary'
            className={`${styles.backButton} d-md-none`}
            onClick={onBack}
            title="Torna alle chat"
          >
            <i className="bi bi-arrow-left"></i>
          </Button>
        )}
        <h3 className={styles.chatName}>{chatName}</h3>
        {!isPrivate && chat.description && (
          <p className={styles.chatDescription}>{chat.description}</p>
        )}
      </div>
      
      <Button
        onClick={onShowInfo}
        title="Informazioni chat"
      >
        <i className="bi bi-three-dots-vertical"></i>
      </Button>
    </div>
  );
}
