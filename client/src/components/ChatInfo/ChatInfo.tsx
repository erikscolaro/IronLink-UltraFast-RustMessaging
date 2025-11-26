// ChatInfo - Pannello laterale con informazioni sulla chat
import { useEffect, useState } from 'react';
import { ChatDTO, ChatType, UserChatMetadataDTO, UserRole, getUserId } from '../../models/types';
import { Button, Spinner, Dropdown, DropdownButton, ButtonGroup } from 'react-bootstrap';
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

  const isPrivate = chat.chat_type === ChatType.Private;

  // Trova il ruolo dell'utente corrente
  const currentUserId = user ? getUserId(user) : undefined;
  const currentUserMember = members.find(m => m.user_id === currentUserId);
  const currentUserRole = currentUserMember?.user_role;
  const isOwner = currentUserRole === UserRole.Owner;
  const isAdmin = currentUserRole === UserRole.Admin;
  const canInvite = isOwner || isAdmin;
  const canRemoveMembers = isOwner || isAdmin;
  const canRemoveAdmins = isOwner;
  const canPromote = isOwner;

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
      } catch (error) {
        console.error('Errore pulizia chat:', error);
        alert('Errore durante la pulizia della chat');
      }
    }
  };

  const handleLeaveChat = async () => {
    if (confirm('Sei sicuro di voler lasciare questa chat? Se sei l\'Owner, devi prima trasferire l\'ownership.')) {
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

    try {
      await api.updateMemberRole(chat.chat_id, userId, UserRole.Admin);
      loadMembers();
    } catch (error) {
      console.error('Errore promozione membro:', error);
      alert('Errore durante la promozione del membro');
    }

  };

  // Retrocedi admin a membro
  const handleDemoteToMember = async (userId: number) => {
    if (!canPromote) {
      alert('Solo l\'owner può retrocedere gli admin');
      return;
    }

    try {
      await api.updateMemberRole(chat.chat_id, userId, UserRole.Member);
      loadMembers();
    } catch (error) {
      console.error('Errore retrocessione membro:', error);
      alert('Errore durante la retrocessione del membro');
    }

  };


  if (!isVisible) return null;

  return (
    <div className={styles.chatInfo}>
      <div className={styles.header}>
        <span className={styles.title}>Info {chat.chat_type==ChatType.Group?chat.title:""}</span>
        <Button variant="primary" onClick={onClose} className="text-white">
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
              <div className='mb-3'>
                <h3 className="text-uppercase text-muted">Dettagli</h3>
                <div>
                  <span className="text-medium text-muted me-3">Nome:</span>
                  <span className="text-medium">{chat.title || 'Senza nome'}</span>
                </div>{chat.description && (
                  <div className="">
                    <span className="text-medium text-muted me-3">Descrizione:</span>
                    <span className="text-medium">{chat.description}</span>
                  </div>
                )}
              </div>
            )}

            {/* Lista membri (solo per gruppi) */}
            {!isPrivate && (
              <div className="mb-3">
                <div className="d-flex justify-content-between align-items-center mb-3">
                  <h3 className="text-uppercase text-muted mb-0">
                    Membri ({members.length})
                  </h3>
                  {canInvite && (
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={handleStartInvite}
                    >
                      <i className="bi bi-person-plus me-1"></i>
                      Invita
                    </Button>
                  )}
                </div>
                <div className={styles.listContainer}>
                  {/* Mostra prima l'utente corrente */}
                  {members
                    .sort((a, b) => {
                      if (a.user_id === currentUserId) return -1;
                      if (b.user_id === currentUserId) return 1;
                      return 0;
                    })
                    .map((member) => {
                      const isCurrentUser = member.user_id === currentUserId;
                      const canRemoveThisMember =
                        !isCurrentUser &&
                        ((member.user_role === UserRole.Member && canRemoveMembers) ||
                          (member.user_role === UserRole.Admin && canRemoveAdmins));
                      const canPromoteThisMember =
                        !isCurrentUser &&
                        member.user_role === UserRole.Member &&
                        canPromote;
                      const canDemoteThisMember =
                        !isCurrentUser &&
                        member.user_role === UserRole.Admin &&
                        canPromote;

                      return (
                        <div
                          key={member.user_id}
                          className={styles.item}
                        >
                          <div className="d-flex justify-content-between align-items-start m-0">
                            <div className="d-flex w-100 align-items-center gap-3">
                              <i className="bi bi-person fs-4"></i>
                              <div>
                                <div className="d-flex align-items-start w-100 m-0">
                                  <span className='text-medium me-2'>
                                    {currentUserId == member.user_id ? "Tu" : (member.username || memberNames.get(member.user_id) || 'Utente')}
                                  </span>
                                  <span className={styles.badge}>{member.user_role || 'Member'}</span>

                                </div>
                                <span className="text-muted text-small">
                                  {member.member_since ?
                                    `Membro dal ${new Date(member.member_since).toLocaleDateString()}` :
                                    'Data non disponibile'}
                                </span>
                              </div>
                            </div>

                            {(canRemoveThisMember || canPromoteThisMember || canDemoteThisMember) && (
                              <div className="d-flex gap-2 align-items-end justify-content-end">
                                <DropdownButton
                                  as={ButtonGroup}
                                  key="left"
                                  id="dropdown-button-drop-up"
                                  drop="up"
                                  variant="primary"
                                  title=""
                                  menuVariant='dark'
                                >
                                  {canPromoteThisMember && (
                                    <Dropdown.Item onClick={() => handlePromoteToAdmin(member.user_id)}><i className="bi bi-arrow-up-circle me-1"></i>
                                      Promuovi</Dropdown.Item>
                                  )}
                                  {canDemoteThisMember && (
                                    <Dropdown.Item onClick={() => handleDemoteToMember(member.user_id)}><i className="bi bi-arrow-down-circle me-1"></i>
                                      Retrocedi</Dropdown.Item>
                                  )}
                                  {canRemoveThisMember && (
                                    <Dropdown.Item onClick={() => handleRemoveMember(member.user_id, member.user_role || UserRole.Member)}
                                    ><i className="bi bi-person-dash me-1"></i>
                                      Rimuovi</Dropdown.Item>
                                  )}



                                  {isOwner && member.user_role === UserRole.Admin && (
                                    <Dropdown.Item
                                      onClick={async () => {
                                        const targetId = member.user_id;
                                        const targetName = member.username || memberNames.get(targetId) || `Utente ${targetId}`;
                                        if (confirm(`Sei sicuro di trasferire la proprietà del gruppo a ${targetName}? Diventerai admin.`)) {
                                          try {
                                            await api.transferOwnership(chat.chat_id, targetId);
                                            alert(`La proprietà è stata trasferita a ${targetName}`);
                                            loadMembers();
                                          } catch (error) {
                                            console.error('Errore trasferimento ownership:', error);
                                            alert('Errore durante il trasferimento della proprietà');
                                          }
                                        }
                                      }}
                                    >
                                      <i className="bi bi-arrow-left-right me-1"></i>
                                      Trasferisci Proprietà
                                    </Dropdown.Item>
                                  )}

                                </DropdownButton>
                              </div>
                            )}
                          </div>




                        </div>
                      );
                    })}
                </div>
              </div>
            )}

            {/* Azioni */}
            <div className="mb-4">
              <div className="d-flex gap-2">
                <Button
                  variant="warn"
                  onClick={handleCleanChat}
                  className="flex-grow-1"
                >
                  <i className="bi bi-eraser me-2"></i>
                  Pulisci chat
                </Button>

                {!isPrivate && (
                  <Button
                    variant="danger"
                    onClick={handleLeaveChat}
                    className="flex-grow-1"
                  >
                    <i className="bi bi-box-arrow-right me-2"></i>
                    Lascia la chat
                  </Button>
                )}
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
