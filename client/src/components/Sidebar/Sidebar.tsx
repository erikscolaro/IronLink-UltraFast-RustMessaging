// Sidebar - Pannello laterale con lista chat e azioni utente
import styles from "./Sidebar.module.css";
import { useState, useEffect } from "react";
import { ChatDTO, ChatType, UserDTO, getUserId, InvitationDTO } from "../../models/types";
import { Button, ListGroup, Form, InputGroup } from "react-bootstrap";
import { useAuth } from "../../context/AuthContext";
import * as api from "../../services/api";

interface SidebarProps {
  chats: ChatDTO[];
  selectedChatId: number | null;
  onSelectChat: (chatId: number) => void;
  onShowProfile: () => void;
  onRefreshChats: () => Promise<void>;
  inviteMode?: {
    chatId: number;
    existingMemberIds: number[];
    onInvite: (userId: number) => Promise<void>;
    onCancel: () => void;
  } | null;
}

type SidebarView = 'chats' | 'createPrivate' | 'createGroup' | 'inviteToGroup';

export default function Sidebar({
  chats,
  selectedChatId,
  onSelectChat,
  onShowProfile,
  onRefreshChats,
  inviteMode,
}: SidebarProps) {
  const { user } = useAuth();
  const [currentView, setCurrentView] = useState<SidebarView>('chats');
  
  // Se inviteMode è attivo, passa automaticamente alla vista invito
  useEffect(() => {
    if (inviteMode) {
      setCurrentView('inviteToGroup');
    } else if (currentView === 'inviteToGroup') {
      setCurrentView('chats');
    }
  }, [inviteMode]);

  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<UserDTO[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [groupName, setGroupName] = useState('');
  const [groupDescription, setGroupDescription] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [privateChatNames, setPrivateChatNames] = useState<Record<number, string>>({});
  const [pendingInvitations, setPendingInvitations] = useState<InvitationDTO[]>([]);
  const [inviterNames, setInviterNames] = useState<Record<number, string>>({});
  const [chatNames, setChatNames] = useState<Record<number, string>>({});

  // Carica gli inviti pending all'avvio e quando le chat cambiano
  useEffect(() => {
    const loadInvitations = async () => {
      try {
        const invites = await api.listPendingInvitations();
        setPendingInvitations(invites);

        // Carica i nomi degli inviter e delle chat
        for (const invite of invites) {
          // Carica il nome dell'inviter
          if (!inviterNames[invite.inviter_id]) {
            try {
              const inviter = await api.getUserById(invite.inviter_id);
              if (inviter && inviter.username) {
                setInviterNames(prev => ({
                  ...prev,
                  [invite.inviter_id]: inviter.username!
                }));
              }
            } catch (error) {
              console.error(`Errore caricamento inviter ${invite.inviter_id}:`, error);
            }
          }

          // Carica il nome della chat
          if (!chatNames[invite.chat_id]) {
            try {
              const chatData = chats.find(c => c.chat_id === invite.chat_id);
              if (chatData && chatData.title) {
                setChatNames(prev => ({
                  ...prev,
                  [invite.chat_id]: chatData.title!
                }));
              }
            } catch (error) {
              console.error(`Errore caricamento chat ${invite.chat_id}:`, error);
            }
          }
        }
      } catch (error) {
        console.error('Errore caricamento inviti:', error);
      }
    };

    if (user) {
      loadInvitations();
    }
  }, [chats, user]);

  // Carica i nomi degli utenti per le chat private
  useEffect(() => {
    const loadPrivateChatNames = async () => {
      if (!user) return;
      
      const currentUserId = getUserId(user);
      const privateChats = chats.filter(chat => 
        chat.chat_type === ChatType.Private && chat.user_list && chat.user_list.length === 2
      );

      for (const chat of privateChats) {
        if (!chat.user_list) continue;
        
        // Trova l'ID dell'altro utente (non quello corrente)
        const otherUserId = chat.user_list.find(id => id !== currentUserId);
        
        if (!otherUserId || privateChatNames[chat.chat_id]) continue;

        try {
          const members = await api.listChatMembers(chat.chat_id);
          const otherUser = members.find((m) => m.user_id !== currentUserId);
          
          if (otherUser && otherUser.username) {
            setPrivateChatNames(prev => ({
              ...prev,
              [chat.chat_id]: otherUser.username!
            }));
          }
        } catch (error) {
          console.error(`Errore caricamento membri chat ${chat.chat_id}:`, error);
        }
      }
    };

    if (chats.length > 0 && user) {
      loadPrivateChatNames();
    }
  }, [chats, user]);

  // Ricerca utenti per chat privata
  const handleSearch = async (query: string) => {
    setSearchQuery(query);
    
    if (query.trim().length < 2) {
      setSearchResults([]);
      return;
    }

    setIsSearching(true);
    try {
      if (!user) return;
      
      const currentUserId = getUserId(user);
      const results = await api.searchUserByUsername(query);
      
      // Trova tutti gli utenti con cui ho già una chat privata
      const existingPrivateChatUserIds = chats
        .filter(chat => chat.chat_type === ChatType.Private && chat.user_list && chat.user_list.length > 0)
        .flatMap(chat => chat.user_list!)
        .filter(id => id !== currentUserId); // Escludi me stesso
      
      // Filtra risultati:
      // 1. Escludi l'utente corrente
      // 2. Escludi utenti con cui esiste già una chat privata
      const filteredResults = results.filter((foundUser) => {
        const userId = getUserId(foundUser);
        return userId !== currentUserId && !existingPrivateChatUserIds.includes(userId);
      });
      
      setSearchResults(filteredResults);
    } catch (error) {
      console.error('Errore ricerca utenti:', error);
    } finally {
      setIsSearching(false);
    }
  };

  // Ricerca utenti per invito a gruppo (filtrata per membri esistenti)
  const handleSearchForInvite = async (query: string) => {
    setSearchQuery(query);
    
    if (query.trim().length < 2) {
      setSearchResults([]);
      return;
    }

    setIsSearching(true);
    try {
      const results = await api.searchUserByUsername(query);
      
      // Filtra risultati escludendo membri esistenti
      const filteredResults = results.filter((foundUser) => {
        const userId = getUserId(foundUser);
        return !inviteMode?.existingMemberIds.includes(userId);
      });
      
      setSearchResults(filteredResults);
    } catch (error) {
      console.error('Errore ricerca utenti:', error);
    } finally {
      setIsSearching(false);
    }
  };

  // Invita utente al gruppo
  const handleInviteUser = async (userId: number) => {
    if (!inviteMode) return;
    
    try {
      await inviteMode.onInvite(userId);
      setSearchQuery('');
      setSearchResults([]);
      inviteMode.onCancel(); // Chiude la modalità invito
    } catch (error) {
      console.error('Errore invito utente:', error);
      alert('Errore durante l\'invito dell\'utente');
    }
  };

  // Accetta invito
  const handleAcceptInvite = async (inviteId: number) => {
    try {
      await api.respondToInvitation(inviteId, 'accept');
      // Rimuovi l'invito dalla lista
      setPendingInvitations(prev => prev.filter(inv => inv.invitation_id !== inviteId));
      // Ricarica le chat per vedere la nuova chat
      await onRefreshChats();
    } catch (error) {
      console.error('Errore accettazione invito:', error);
      alert('Errore durante l\'accettazione dell\'invito');
    }
  };

  // Rifiuta invito
  const handleDeclineInvite = async (inviteId: number) => {
    try {
      await api.respondToInvitation(inviteId, 'decline');
      // Rimuovi l'invito dalla lista
      setPendingInvitations(prev => prev.filter(inv => inv.invitation_id !== inviteId));
    } catch (error) {
      console.error('Errore rifiuto invito:', error);
      alert('Errore durante il rifiuto dell\'invito');
    }
  };

  // Crea chat privata con utente selezionato
  const handleCreatePrivateChat = async (userId: number) => {
    if (!user) {
      console.error('Utente non autenticato');
      return;
    }

    try {
      const currentUserId = user.id || user.user_id;
      if (!currentUserId) {
        console.error('ID utente corrente non disponibile');
        return;
      }

      const newChat = await api.createChat({
        chat_type: ChatType.Private,
        user_list: [currentUserId, userId] // Include entrambi gli utenti
      });
      
      // Ricarica la lista per vedere la nuova chat
      await onRefreshChats();
      
      onSelectChat(newChat.chat_id);
      setCurrentView('chats');
      setSearchQuery('');
      setSearchResults([]);
    } catch (error) {
      console.error('Errore creazione chat privata:', error);
      alert('Errore durante la creazione della chat privata');
    }
  };

  // Crea chat di gruppo
  const handleCreateGroup = async () => {
    if (!groupName.trim()) {
      alert('Inserisci un nome per il gruppo');
      return;
    }

    setIsCreating(true);
    try {
      const newChat = await api.createChat({
        chat_type: ChatType.Group,
        title: groupName,
        description: groupDescription || undefined
      });
      
      // Ricarica la lista per vedere il nuovo gruppo
      await onRefreshChats();
      
      onSelectChat(newChat.chat_id);
      setCurrentView('chats');
      setGroupName('');
      setGroupDescription('');
    } catch (error) {
      console.error('Errore creazione gruppo:', error);
      alert('Errore durante la creazione del gruppo');
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <div className={styles.sidebar}>
      {/* Header con info utente e azioni */}
      <div className={styles.header}>
        <div className="d-flex align-items-center gap-2">
          <i className="bi bi-person-circle fs-2"></i>
          <span className="fw-bold">{user?.username || 'Utente'}</span>
        </div>
        <Button
          variant="link"
          onClick={onShowProfile}
          title="Profilo e impostazioni"
          className="text-white p-1"
        >
          <i className="bi bi-gear fs-4"></i>
        </Button>
      </div>

      {/* Contenuto principale */}
      <div className={styles.content}>
        {currentView === 'chats' && (
          <>
            {/* Lista chat */}
            <div className={styles.listContainer}>
              {/* Inviti pending */}
              {pendingInvitations.length > 0 && (
                <div className="mb-3">
                  <div className="px-3 py-2">
                    <small className="text-uppercase text-muted fw-bold">
                      Inviti Pending ({pendingInvitations.length})
                    </small>
                  </div>
                  {pendingInvitations.map((invite) => (
                    <div key={invite.invitation_id} className={styles.inviteItem}>
                      <div className="d-flex align-items-start gap-2 mb-2">
                        <i className="bi bi-envelope-fill text-warning fs-5"></i>
                        <div className="flex-grow-1">
                          <div className="fw-bold">
                            {chatNames[invite.chat_id] || `Gruppo ${invite.chat_id}`}
                          </div>
                          <small className="text-muted">
                            Invitato da {inviterNames[invite.inviter_id] || `Utente ${invite.inviter_id}`}
                          </small>
                        </div>
                      </div>
                      <div className={styles.inviteActions}>
                        <Button
                          size="sm"
                          variant="success"
                          onClick={() => handleAcceptInvite(invite.invitation_id)}
                          className={styles.acceptButton}
                        >
                          <i className="bi bi-check-circle me-1"></i>
                          Accetta
                        </Button>
                        <Button
                          size="sm"
                          variant="danger"
                          onClick={() => handleDeclineInvite(invite.invitation_id)}
                          className={styles.declineButton}
                        >
                          <i className="bi bi-x-circle me-1"></i>
                          Rifiuta
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {chats.length === 0 ? (
                <div className="text-center py-5 text-muted">
                  <i className="bi bi-chat-dots mb-3" style={{ fontSize: '3rem' }}></i>
                  <p>Nessuna chat disponibile</p>
                  <small>Crea una nuova chat per iniziare</small>
                </div>
              ) : (
                <ListGroup variant="flush">
                  {chats.map((chat) => {
                    const isPrivate = chat.chat_type === ChatType.Private;
                    const displayName = isPrivate 
                      ? (privateChatNames[chat.chat_id] || `Chat ${chat.chat_id}`)
                      : (chat.title || `Gruppo ${chat.chat_id}`);
                    
                    return (
                      <ListGroup.Item
                        key={chat.chat_id}
                        action
                        active={selectedChatId === chat.chat_id}
                        onClick={() => onSelectChat(chat.chat_id)}
                        className="bg-transparent text-white border-0"
                      >
                        <div className="d-flex align-items-center gap-2">
                          <i className={`bi ${isPrivate ? 'bi-person-circle' : 'bi-people-fill'} fs-5`}></i>
                          <div className="flex-grow-1">
                            <div className="fw-bold">
                              {displayName}
                            </div>
                            {chat.description && !isPrivate && (
                              <small className="text-muted text-truncate d-block">
                                {chat.description}
                              </small>
                            )}
                          </div>
                        </div>
                      </ListGroup.Item>
                    );
                  })}
                </ListGroup>
              )}
            </div>
          </>
        )}

        {currentView === 'createPrivate' && (
          <div className={styles.createContainer}>
            <div className="d-flex align-items-center justify-content-between mb-3">
              <h5 className="mb-0">Nuova Chat Privata</h5>
              <Button 
                variant="link" 
                onClick={() => {
                  setCurrentView('chats');
                  setSearchQuery('');
                  setSearchResults([]);
                }}
                className="text-white p-0"
              >
                <i className="bi bi-x-lg"></i>
              </Button>
            </div>

            <InputGroup className="mb-3">
              <Form.Control
                placeholder="Cerca utente..."
                value={searchQuery}
                onChange={(e) => handleSearch(e.target.value)}
              />
            </InputGroup>

            <div className={styles.searchResults}>
              {isSearching && <div className="text-center py-3">Ricerca...</div>}
              {!isSearching && searchQuery.length >= 2 && searchResults.length === 0 && (
                <div className="text-center py-3 text-muted">Nessun utente trovato</div>
              )}
              {searchResults.map((foundUser) => (
                <div
                  key={getUserId(foundUser)}
                  className={styles.searchResultItem}
                  onClick={() => handleCreatePrivateChat(getUserId(foundUser))}
                >
                  <i className="bi bi-person-circle me-2"></i>
                  {foundUser.username}
                </div>
              ))}
            </div>
          </div>
        )}

        {currentView === 'createGroup' && (
          <div className={styles.createContainer}>
            <div className="d-flex align-items-center justify-content-between mb-3">
              <h5 className="mb-0">Nuovo Gruppo</h5>
              <Button 
                variant="link" 
                onClick={() => {
                  setCurrentView('chats');
                  setGroupName('');
                  setGroupDescription('');
                }}
                className="text-white p-0"
              >
                <i className="bi bi-x-lg"></i>
              </Button>
            </div>

            <Form.Group className="mb-3">
              <Form.Label>Nome Gruppo</Form.Label>
              <Form.Control
                placeholder="Inserisci nome del gruppo"
                value={groupName}
                onChange={(e) => setGroupName(e.target.value)}
              />
            </Form.Group>

            <Form.Group className="mb-3">
              <Form.Label>Descrizione (opzionale)</Form.Label>
              <Form.Control
                as="textarea"
                rows={3}
                placeholder="Inserisci descrizione"
                value={groupDescription}
                onChange={(e) => setGroupDescription(e.target.value)}
              />
            </Form.Group>

            <Button 
              variant="danger" 
              className="w-100"
              onClick={handleCreateGroup}
              disabled={isCreating || !groupName.trim()}
            >
              {isCreating ? 'Creazione...' : 'Crea Gruppo'}
            </Button>
          </div>
        )}

        {currentView === 'inviteToGroup' && inviteMode && (
          <div className={styles.createContainer}>
            <div className="d-flex align-items-center justify-content-between mb-3">
              <h5 className="mb-0">Invita Utente</h5>
              <Button 
                variant="link" 
                onClick={() => {
                  inviteMode.onCancel();
                  setSearchQuery('');
                  setSearchResults([]);
                }}
                className="text-white p-0"
              >
                <i className="bi bi-x-lg"></i>
              </Button>
            </div>

            <InputGroup className="mb-3">
              <Form.Control
                placeholder="Cerca utente..."
                value={searchQuery}
                onChange={(e) => handleSearchForInvite(e.target.value)}
              />
            </InputGroup>

            <div className={styles.searchResults}>
              {isSearching && <div className="text-center py-3">Ricerca...</div>}
              {!isSearching && searchQuery.length >= 2 && searchResults.length === 0 && (
                <div className="text-center py-3 text-muted">Nessun utente trovato</div>
              )}
              {searchResults.map((foundUser) => (
                <div
                  key={getUserId(foundUser)}
                  className={styles.searchResultItem}
                  onClick={() => handleInviteUser(getUserId(foundUser))}
                >
                  <i className="bi bi-person-circle me-2"></i>
                  {foundUser.username}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Pulsanti creazione chat (solo quando mostra lista chat) */}
      {currentView === 'chats' && (
        <div className={styles.createButtons}>
          <Button
            variant="outline-light"
            className="w-100 mb-2"
            onClick={() => setCurrentView('createPrivate')}
          >
            <i className="bi bi-person-plus me-2"></i>
            Nuova Chat Privata
          </Button>
          <Button
            variant="outline-light"
            className="w-100"
            onClick={() => setCurrentView('createGroup')}
          >
            <i className="bi bi-people me-2"></i>
            Nuovo Gruppo
          </Button>
        </div>
      )}
    </div>
  );
}
