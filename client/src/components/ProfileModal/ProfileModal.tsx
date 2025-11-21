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
      logout();
      onHide();
      navigate('/login');
    
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
      logout();
    }
  };



  return (
    <Modal
      className={styles.modal}
      show={show}
      onHide={onHide}
      centered
      contentClassName={`${styles.modalContent} bg-dark text-light`}
    >
      <Modal.Header closeButton className={`${styles.modalHeader} border-0 bg-dark text-light`}>
      <div className="d-flex align-items-center">
        <div
        className={`${styles.userAvatar} rounded-circle bg-secondary d-flex justify-content-center align-items-center me-3`}
        style={{ width: 56, height: 56 }}
        >
        <i className="bi bi-person-fill fs-3 text-light"></i>
        </div>
        <div>
        <Modal.Title className="mb-0 text-light">
          {user?.username || 'Utente'}
        </Modal.Title>
        <div className="small text-secondary">
          ID: <span className="badge bg-dark text-light align-middle">{user?.user_id}</span>
        </div>
        </div>
      </div>
      </Modal.Header>

      <Modal.Body className={`${styles.modalBody} pt-0 bg-dark text-light`}>
      {error && (
        <Alert
        variant="danger"
        dismissible
        onClose={() => setError(null)}
        className="bg-danger text-light border-0"
        >
        {error}
        </Alert>
      )}

      <ListGroup variant="flush" className="mb-3">
        <ListGroup.Item
        action
        onClick={handleLogout}
        className="d-flex align-items-center bg-dark border-bottom border-secondary"
        >
        <div className="me-3 text-primary">
          <i className="bi bi-box-arrow-right fs-5 text-light"></i>
        </div>
        <div className="flex-grow-1">
          <div className="fw-semibold text-light">Esci dall'app</div>
          <div className="small text-secondary">Termina la sessione corrente</div>
        </div>
        <div className="text-secondary small">&rsaquo;</div>
        </ListGroup.Item>

        <ListGroup.Item
        action
        onClick={() => { if (!isDeleting) handleDeleteAccount(); }}
        className="d-flex align-items-center bg-dark border-bottom border-secondary"
        aria-disabled={isDeleting}
        >
        <div className="me-3">
          <i className="bi bi-trash-fill fs-5 text-danger"></i>
        </div>
        <div className="flex-grow-1">
          <div className="fw-semibold text-light">
          {isDeleting ? (
            <>
            <span className="spinner-border spinner-border-sm me-2 text-light" role="status" aria-hidden="true"></span>
            Eliminazione in corso...
            </>
          ) : 'Elimina account'}
          </div>
          <div className="small text-secondary">Questa azione è irreversibile</div>
        </div>
        <div className="small text-secondary">&rsaquo;</div>
        </ListGroup.Item>
      </ListGroup>

      <div className="text-center small text-secondary">
        I tuoi messaggi rimarranno visibili come "Deleted User". 
      </div>
      </Modal.Body>

      <Modal.Footer className="border-0 bg-dark">
      <Button variant="outline-light" onClick={onHide}>
        Chiudi
      </Button>
      </Modal.Footer>
    </Modal>
  );
}
