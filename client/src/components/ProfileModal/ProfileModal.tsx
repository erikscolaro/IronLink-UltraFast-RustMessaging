// ProfileModal - Modale per gestire il profilo utente
import { useState } from 'react';
import { Modal, Button, ListGroup, Alert } from 'react-bootstrap';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../../context/AuthContext';
import * as api from '../../services/api';
import styles from './ProfileModal.module.css';

interface ProfileModalProps {
  show: boolean;
  onHide: () => void;
}

export default function ProfileModal({ show, onHide }: ProfileModalProps) {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [isDeleting, setIsDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLogout = () => {
    if (confirm('Sei sicuro di voler uscire?')) {
      logout();
      onHide();
      navigate('/login');
    }
  };

  const handleDeleteAccount = async () => {
    const confirmMessage = 
      'ATTENZIONE: Questa azione eliminerà permanentemente il tuo account.\n\n' +
      '- Verrai rimosso da tutte le chat\n' +
      '- I tuoi messaggi rimarranno visibili come "Deleted User"\n' +
      '- Questa azione NON può essere annullata\n\n' +
      'Sei assolutamente sicuro di voler procedere?';
    
    if (!confirm(confirmMessage)) {
      return;
    }

    // Seconda conferma
    if (!confirm('Ultima conferma: eliminare definitivamente l\'account?')) {
      return;
    }

    setIsDeleting(true);
    setError(null);

    try {
      await api.deleteMyAccount();
      // Il logout viene fatto automaticamente da deleteMyAccount
      onHide();
      navigate('/login');
    } catch (err) {
      console.error('Errore eliminazione account:', err);
      setError(err instanceof Error ? err.message : 'Errore durante l\'eliminazione dell\'account');
    } finally {
      setIsDeleting(false);
    }
  };

  return (
    <Modal show={show} onHide={onHide} centered>
      <Modal.Header closeButton className={styles.modalHeader}>
        <Modal.Title>
          <i className="bi bi-person-circle me-2"></i>
          Profilo
        </Modal.Title>
      </Modal.Header>

      <Modal.Body>
        {/* Informazioni utente */}
        <div className={styles.userInfo}>
          <div className={styles.userAvatar}>
            <i className="bi bi-person-circle"></i>
          </div>
          <h4 className="mb-0">{user?.username}</h4>
          <p className="text-muted small">ID: {user?.user_id}</p>
        </div>

        {error && (
          <Alert variant="danger" dismissible onClose={() => setError(null)}>
            {error}
          </Alert>
        )}

        {/* Azioni */}
        <ListGroup variant="flush" className="mt-3">
          <ListGroup.Item 
            action 
            onClick={handleLogout}
            className={styles.actionItem}
          >
            <i className="bi bi-box-arrow-right me-2"></i>
            Esci dall'app
          </ListGroup.Item>

          <ListGroup.Item 
            action 
            disabled
            className={`${styles.actionItem} ${styles.disabled}`}
          >
            <i className="bi bi-key me-2"></i>
            Modifica password
            <small className="text-muted d-block">Funzionalità non ancora disponibile</small>
          </ListGroup.Item>

          <ListGroup.Item 
            action 
            onClick={handleDeleteAccount}
            className={`${styles.actionItem} ${styles.danger}`}
            disabled={isDeleting}
          >
            <i className="bi bi-trash me-2"></i>
            {isDeleting ? 'Eliminazione in corso...' : 'Elimina account'}
            <small className="text-muted d-block">
              Questa azione è irreversibile
            </small>
          </ListGroup.Item>
        </ListGroup>
      </Modal.Body>

      <Modal.Footer>
        <Button variant="secondary" onClick={onHide}>
          Chiudi
        </Button>
      </Modal.Footer>
    </Modal>
  );
}
