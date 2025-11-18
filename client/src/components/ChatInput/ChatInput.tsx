// ChatInput - Componente per l'invio di messaggi
import { useState } from 'react';
import { Form, Button, InputGroup } from 'react-bootstrap';
import styles from './ChatInput.module.css';

interface ChatInputProps {
  onSendMessage: (content: string) => void;
  disabled?: boolean;
}

export default function ChatInput({ onSendMessage, disabled = false }: ChatInputProps) {
  const [message, setMessage] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (message.trim() && !disabled) {
      onSendMessage(message.trim());
      setMessage('');
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
  };

  return (
    <div className={styles.chatInputContainer}>
      <Form onSubmit={handleSubmit}>
        <InputGroup>
          <Form.Control
            as="textarea"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            onKeyDown={handleKeyPress}
            placeholder="Scrivi un messaggio..."
            disabled={disabled}
            rows={1}
            style={{ resize: 'none', maxHeight: '120px' }}
          />
          <Button
            variant="danger"
            type="submit"
            disabled={disabled || !message.trim()}
          >
            <i className="bi bi-send-fill"></i>
          </Button>
        </InputGroup>
      </Form>
    </div>
  );
}
