// Sidebar - Pannello laterale con lista chat e azioni utente
import styles from "./Sidebar.module.css";
import { useState, useEffect } from "react";
import { ChatDTO, ChatType, UserDTO, getUserId } from "../../models/types";
import { Button, ListGroup, Form, InputGroup } from "react-bootstrap";
import { useAuth } from "../../context/AuthContext";
import * as api from "../../services/api";

interface SidebarProps {
  chats: ChatDTO[];
  selectedChatId: number | null;
  onSelectChat: (chatId: number) => void;
  onShowProfile: () => void;
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

  // Ricerca utenti per chat privata
  const handleSearch = async (query: string) => {
    setSearchQuery(query);
    
    if (query.trim().length < 2) {
      setSearchResults([]);
      return;
    }

    setIsSearching(true);
    try {
      const results = await api.searchUserByUsername(query);
      console.log('Search results:', results);
      
      // Filtra risultati:
      // 1. Escludi l'utente corrente
      // 2. Escludi utenti con cui esiste già una chat privata
      const filteredResults = results.filter((foundUser) => {
        // Escludi solo se stesso
        return getUserId(foundUser) !== user?.user_id;
      });
      
      console.log('Filtered results:', filteredResults);
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
      console.log('Invite search results:', results);
      
      // Filtra risultati escludendo membri esistenti
      const filteredResults = results.filter((foundUser) => {
        const userId = getUserId(foundUser);
        const isExisting = inviteMode?.existingMemberIds.includes(userId);
        console.log(`User ${foundUser.username} (${userId}): existing=${isExisting}`);
        return !isExisting;
      });
      
      console.log('Filtered invite results:', filteredResults);
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
              {chats.length === 0 ? (
                <div className="text-center py-5 text-muted">
                  <i className="bi bi-chat-dots mb-3" style={{ fontSize: '3rem' }}></i>
                  <p>Nessuna chat disponibile</p>
                  <small>Crea una nuova chat per iniziare</small>
                </div>
              ) : (
                <ListGroup variant="flush">
                  {chats.map((chat) => (
                    <ListGroup.Item
                      key={chat.chat_id}
                      action
                      active={selectedChatId === chat.chat_id}
                      onClick={() => onSelectChat(chat.chat_id)}
                      className="bg-transparent text-white border-0"
                    >
                      <div className="fw-bold">
                        {chat.title || `Chat ${chat.chat_id}`}
                      </div>
                      {chat.description && (
                        <small className="text-muted text-truncate d-block">
                          {chat.description}
                        </small>
                      )}
                    </ListGroup.Item>
                  ))}
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
