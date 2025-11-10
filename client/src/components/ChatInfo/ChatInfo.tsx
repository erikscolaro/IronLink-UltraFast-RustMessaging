// ChatInfo - Pannello laterale con informazioni sulla chat
import { useEffect, useState } from 'react';
import { ChatDTO, ChatType, UserChatMetadataDTO, UserRole } from '../../models/types';
import { Button, ListGroup, Badge, Spinner, Form } from 'react-bootstrap';
import { useAuth } from '../../context/AuthContext';
import * as api from '../../services/api';
import styles from './ChatInfo.module.css';

interface ChatInfoProps {
  chat: ChatDTO;
  isVisible: boolean;
  onClose: () => void;
  onStartInvite: (chatId: number, memberIds: number[], onInvite: (userId: number) => Promise<void>) => void;
  onChatLeft?: () => void; // Callback quando l'utente esce dalla chat
  onChatCleaned?: () => void; // Callback quando la chat viene pulita
}

export default function ChatInfo({ chat, isVisible, onClose, onStartInvite, onChatLeft, onChatCleaned }: ChatInfoProps) {
  const { user } = useAuth();
  const [members, setMembers] = useState<UserChatMetadataDTO[]>([]);
  const [memberNames, setMemberNames] = useState<Map<number, string>>(new Map());
  const [isLoading, setIsLoading] = useState(true);
  const [showTransferOwnership, setShowTransferOwnership] = useState(false);
  const [selectedAdminForTransfer, setSelectedAdminForTransfer] = useState<number | null>(null);
  
  const isPrivate = chat.chat_type === ChatType.Private;
  
  // Trova il ruolo dell'utente corrente
  const currentUserMember = members.find(m => m.user_id === user?.user_id);
  const currentUserRole = currentUserMember?.user_role;
  const isOwner = currentUserRole === UserRole.Owner;
  const isAdmin = currentUserRole === UserRole.Admin;
  const canInvite = isOwner || isAdmin;
  const canRemoveMembers = isOwner || isAdmin;
  const canRemoveAdmins = isOwner;
  const canPromote = isOwner;

  // Debug: log dei permessi
  console.log('ChatInfo Debug:', {
    userId: user?.user_id,
    currentUserMember,
    currentUserRole,
    isOwner,
    isAdmin,
    canInvite,
    members: members.length
  });

  const loadMembers = async () => {
    setIsLoading(true);
    try {
      const chatMembers = await api.listChatMembers(chat.chat_id);
      setMembers(chatMembers);

      // Carica i nomi degli utenti
      const names = new Map<number, string>();
      for (const member of chatMembers) {
        try {
          const userData = await api.getUserById(member.user_id);
          const userId = userData.id || userData.user_id;
          const username = userData.username;
          if (userId && username) {
            names.set(userId, username);
          }
        } catch (error) {
          console.error(`Errore caricamento utente ${member.user_id}:`, error);
        }
      }
      setMemberNames(names);
    } catch (error) {
      console.error('Errore caricamento membri:', error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    if (!isVisible) return;
    loadMembers();
  }, [chat.chat_id, isVisible]);

  const handleCleanChat = async () => {
    if (confirm('Vuoi pulire questa chat? I messaggi precedenti non saranno più visibili.')) {
      try {
        await api.cleanChat(chat.chat_id);
        if (onChatCleaned) onChatCleaned(); // Notifica che la chat è stata pulita
        alert('Chat pulita con successo');
      } catch (error) {
        console.error('Errore pulizia chat:', error);
        alert('Errore durante la pulizia della chat');
      }
    }
  };

  const handleDeleteChat = async () => {
    const message = isPrivate 
      ? 'Eliminare questa chat rimuoverà definitivamente tutti i tuoi metadati e messaggi. Continuare?'
      : 'Eliminare questa chat ti farà uscire dal gruppo e rimuoverà tutti i tuoi dati. Continuare?';
      
    if (confirm(message)) {
      try {
        await api.leaveChat(chat.chat_id);
        onClose();
        if (onChatLeft) onChatLeft(); // Notifica che la chat è stata lasciata
      } catch (error) {
        console.error('Errore eliminazione chat:', error);
        alert('Errore durante l\'eliminazione della chat');
      }
    }
  };

  const handleLeaveChat = async () => {
    if (confirm('Sei sicuro di voler uscire da questa chat?')) {
      try {
        await api.leaveChat(chat.chat_id);
        onClose();
        if (onChatLeft) onChatLeft(); // Notifica che la chat è stata lasciata
      } catch (error) {
        console.error('Errore uscita chat:', error);
        alert('Errore durante l\'uscita dalla chat');
      }
    }
  };

  // Avvia modalità invito nella sidebar
  const handleStartInvite = () => {
    if (!canInvite) {
      alert('Solo gli admin e l\'owner possono invitare utenti');
      return;
    }
    const memberIds = members.map(m => m.user_id);
    onStartInvite(chat.chat_id, memberIds, async (userId: number) => {
      await api.inviteToChat(chat.chat_id, userId);
      alert('Invito inviato con successo!');
      // Ricarica membri per aggiornare la lista
      loadMembers();
    });
  };

  // Rimuovi membro dal gruppo
  const handleRemoveMember = async (userId: number, memberRole: UserRole) => {
    // Verifica permessi
    if (memberRole === UserRole.Admin && !canRemoveAdmins) {
      alert('Solo l\'owner può rimuovere gli admin');
      return;
    }
    if (memberRole === UserRole.Member && !canRemoveMembers) {
      alert('Solo gli admin e l\'owner possono rimuovere i membri');
      return;
    }
    if (memberRole === UserRole.Owner) {
      alert('Non puoi rimuovere l\'owner');
      return;
    }

    const memberName = memberNames.get(userId) || 'questo utente';
    if (confirm(`Sei sicuro di voler rimuovere ${memberName} dal gruppo?`)) {
      try {
        await api.removeMember(chat.chat_id, userId);
        alert(`${memberName} è stato rimosso dal gruppo`);
        loadMembers();
      } catch (error) {
        console.error('Errore rimozione membro:', error);
        alert('Errore durante la rimozione del membro');
      }
    }
  };

  // Promuovi membro a admin
  const handlePromoteToAdmin = async (userId: number) => {
    if (!canPromote) {
      alert('Solo l\'owner può promuovere gli utenti ad admin');
      return;
    }

    const memberName = memberNames.get(userId) || 'questo utente';
    if (confirm(`Vuoi promuovere ${memberName} ad Admin?`)) {
      try {
        await api.updateMemberRole(chat.chat_id, userId, UserRole.Admin);
        alert(`${memberName} è stato promosso ad Admin`);
        loadMembers();
      } catch (error) {
        console.error('Errore promozione membro:', error);
        alert('Errore durante la promozione del membro');
      }
    }
  };

  // Trasferisci ownership
  const handleTransferOwnership = async () => {
    if (!selectedAdminForTransfer) {
      alert('Seleziona un admin a cui trasferire la proprietà');
      return;
    }

    const newOwnerName = memberNames.get(selectedAdminForTransfer) || 'l\'utente selezionato';
    if (confirm(`Sei sicuro di voler trasferire la proprietà del gruppo a ${newOwnerName}? Diventerai admin.`)) {
      try {
        await api.transferOwnership(chat.chat_id, selectedAdminForTransfer);
        alert(`La proprietà è stata trasferita a ${newOwnerName}`);
        setShowTransferOwnership(false);
        setSelectedAdminForTransfer(null);
        loadMembers();
      } catch (error) {
        console.error('Errore trasferimento ownership:', error);
        alert('Errore durante il trasferimento della proprietà');
      }
    }
  };

  if (!isVisible) return null;

  return (
    <div className={styles.chatInfo}>
      <div className={styles.header}>
        <h3 className={styles.title}>Informazioni Chat</h3>
        <Button variant="link" onClick={onClose} className="text-white">
          <i className="bi bi-x-lg"></i>
        </Button>
      </div>

      <div className={styles.content}>
        {isLoading ? (
          <div className="text-center py-5">
            <Spinner animation="border" variant="light" />
            <p className="mt-2">Caricamento...</p>
          </div>
        ) : (
          <>
            {/* Titolo e Descrizione (solo per gruppi) */}
            {!isPrivate && (
              <div className="mb-4">
                <h5 className="text-uppercase text-muted mb-3">Dettagli</h5>
                <div className="mb-2">
                  <small className="text-muted">Nome:</small>
                  <p className="mb-0">{chat.title || 'Senza nome'}</p>
                </div>
                {chat.description && (
                  <div className="mb-2">
                    <small className="text-muted">Descrizione:</small>
                    <p className="mb-0">{chat.description}</p>
                  </div>
                )}
              </div>
            )}

            {/* Lista membri (solo per gruppi) */}
            {!isPrivate && (
              <div className="mb-4">
                <div className="d-flex justify-content-between align-items-center mb-3">
                  <h5 className="text-uppercase text-muted mb-0">
                    Membri ({members.length})
                  </h5>
                  {canInvite && (
                    <Button
                      variant="outline-light"
                      size="sm"
                      onClick={handleStartInvite}
                    >
                      <i className="bi bi-person-plus me-1"></i>
                      Invita
                    </Button>
                  )}
                </div>
                <ListGroup variant="flush">
                  {/* Mostra prima l'utente corrente */}
                  {members
                    .sort((a, b) => {
                      if (a.user_id === user?.user_id) return -1;
                      if (b.user_id === user?.user_id) return 1;
                      return 0;
                    })
                    .map((member) => {
                      const isCurrentUser = member.user_id === user?.user_id;
                      const canRemoveThisMember = 
                        !isCurrentUser && 
                        ((member.user_role === UserRole.Member && canRemoveMembers) ||
                         (member.user_role === UserRole.Admin && canRemoveAdmins));
                      const canPromoteThisMember = 
                        !isCurrentUser && 
                        member.user_role === UserRole.Member && 
                        canPromote;

                      return (
                        <ListGroup.Item 
                          key={member.user_id} 
                          className="bg-transparent text-white"
                        >
                          <div className="d-flex justify-content-between align-items-start mb-2">
                            <div className="d-flex align-items-center gap-2">
                              <i className="bi bi-person-circle fs-4"></i>
                              <div>
                                <div>
                                  {member.username || memberNames.get(member.user_id) || 'Utente'}
                                  {isCurrentUser && (
                                    <Badge bg="secondary" className="ms-2">Tu</Badge>
                                  )}
                                </div>
                                <small className="text-muted">
                                  {member.member_since ? 
                                    `Membro dal ${new Date(member.member_since).toLocaleDateString()}` : 
                                    'Data non disponibile'}
                                </small>
                              </div>
                            </div>
                            <Badge bg="danger">{member.user_role || 'Member'}</Badge>
                          </div>
                          
                          {/* Azioni membro */}
                          {(canRemoveThisMember || canPromoteThisMember) && (
                            <div className="d-flex gap-2">
                              {canPromoteThisMember && (
                                <Button
                                  size="sm"
                                  variant="outline-warning"
                                  onClick={() => handlePromoteToAdmin(member.user_id)}
                                >
                                  <i className="bi bi-arrow-up-circle me-1"></i>
                                  Promuovi ad Admin
                                </Button>
                              )}
                              {canRemoveThisMember && (
                                <Button
                                  size="sm"
                                  variant="outline-danger"
                                  onClick={() => handleRemoveMember(member.user_id, member.user_role || UserRole.Member)}
                                >
                                  <i className="bi bi-person-dash me-1"></i>
                                  Rimuovi
                                </Button>
                              )}
                            </div>
                          )}
                        </ListGroup.Item>
                      );
                    })}
                </ListGroup>

                {/* Trasferisci Proprietà (solo Owner) */}
                {isOwner && !showTransferOwnership && (
                  <Button
                    variant="outline-warning"
                    size="sm"
                    className="mt-3 w-100"
                    onClick={() => setShowTransferOwnership(true)}
                  >
                    <i className="bi bi-arrow-left-right me-2"></i>
                    Trasferisci Proprietà
                  </Button>
                )}

                {/* Form trasferimento proprietà */}
                {isOwner && showTransferOwnership && (
                  <div className="mt-3 p-3 border border-warning rounded">
                    <h6 className="text-warning mb-3">Trasferisci Proprietà</h6>
                    <p className="small text-muted mb-3">
                      Seleziona un admin a cui trasferire la proprietà del gruppo. 
                      Diventerai admin dopo il trasferimento.
                    </p>
                    <Form.Select
                      value={selectedAdminForTransfer || ''}
                      onChange={(e) => setSelectedAdminForTransfer(Number(e.target.value))}
                      className="mb-3 bg-dark text-white"
                    >
                      <option value="">Seleziona un admin...</option>
                      {members
                        .filter(m => m.user_role === UserRole.Admin && m.user_id !== user?.user_id)
                        .map(m => (
                          <option key={m.user_id} value={m.user_id}>
                            {m.username || memberNames.get(m.user_id) || `Utente ${m.user_id}`}
                          </option>
                        ))}
                    </Form.Select>
                    <div className="d-flex gap-2">
                      <Button
                        variant="warning"
                        size="sm"
                        onClick={handleTransferOwnership}
                        disabled={!selectedAdminForTransfer}
                      >
                        Conferma Trasferimento
                      </Button>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => {
                          setShowTransferOwnership(false);
                          setSelectedAdminForTransfer(null);
                        }}
                      >
                        Annulla
                      </Button>
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* Azioni */}
            <div className="mb-4">
              <h5 className="text-uppercase text-muted mb-3">Azioni</h5>
              <div className="d-grid gap-2">
                {/* Pulisci chat - disponibile per tutti i tipi di chat */}
                <Button
                  variant="outline-warning"
                  onClick={handleCleanChat}
                >
                  <i className="bi bi-eraser me-2"></i>
                  Pulisci chat
                </Button>
                
                {/* Azioni per chat di gruppo */}
                {!isPrivate && (
                  <>
                    <Button
                      variant="outline-light"
                      onClick={handleLeaveChat}
                    >
                      <i className="bi bi-box-arrow-right me-2"></i>
                      Esci dalla chat
                    </Button>
                    <Button
                      variant="danger"
                      onClick={handleDeleteChat}
                    >
                      <i className="bi bi-trash me-2"></i>
                      Elimina chat
                    </Button>
                  </>
                )}
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
