// WebSocketContext - Gestisce la connessione WebSocket per messaggi real-time tramite Tauri
import { createContext, useContext, useState, useEffect, useCallback, ReactNode, useRef } from 'react';
import { MessageDTO } from '../models/types';
import { useAuth } from './AuthContext';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const WS_URL = import.meta.env.VITE_WS_URL || 'ws://localhost:8080/ws';

interface WebSocketContextType {
  isConnected: boolean;
  sendMessage: (message: MessageDTO) => void;
  subscribeToChat: (chatId: number, callback: (message: MessageDTO) => void) => () => void;
  onError: (callback: (error: string) => void) => () => void;
  onChatAdded: (callback: (chatId: number) => void) => () => void;
  onChatRemoved: (callback: (chatId: number) => void) => () => void;
}

const WebSocketContext = createContext<WebSocketContextType | undefined>(undefined);

export function useWebSocket() {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error('useWebSocket deve essere usato all\'interno di WebSocketProvider');
  }
  return context;
}

interface WebSocketProviderProps {
  children: ReactNode;
}

export function WebSocketProvider({ children }: WebSocketProviderProps) {
  const { isAuthenticated } = useAuth();
  const [isConnected, setIsConnected] = useState(false);
  const chatCallbacksRef = useRef<Map<number, Set<(message: MessageDTO) => void>>>(new Map());
  const errorCallbacksRef = useRef<Set<(error: string) => void>>(new Set());
  const chatAddedCallbacksRef = useRef<Set<(chatId: number) => void>>(new Set());
  const chatRemovedCallbacksRef = useRef<Set<(chatId: number) => void>>(new Set());
  const reconnectTimeoutRef = useRef<number | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const MAX_RECONNECT_ATTEMPTS = 5;
  const RECONNECT_DELAY = 3000;

  const connect = useCallback(async () => {
    if (!isAuthenticated) {
      return;
    }

    const token = localStorage.getItem('token');
    if (!token) {
      return;
    }

    try {
      console.log('Connessione WebSocket tramite Tauri...');
      
      // Usa il comando Tauri per connettersi
      await invoke('connect_websocket', {
        wsUrl: WS_URL,
        token: token,
      });

      console.log('Comando WebSocket inviato');
    } catch (error) {
      console.error('Errore connessione WebSocket:', error);
      errorCallbacksRef.current.forEach(callback => 
        callback('Errore di connessione al server')
      );
      
      // Tentativo di riconnessione
      if (reconnectAttemptsRef.current < MAX_RECONNECT_ATTEMPTS) {
        reconnectAttemptsRef.current++;
        console.log(`Tentativo di riconnessione ${reconnectAttemptsRef.current}/${MAX_RECONNECT_ATTEMPTS}...`);
        reconnectTimeoutRef.current = setTimeout(connect, RECONNECT_DELAY) as unknown as number;
      }
    }
  }, [isAuthenticated]);

  useEffect(() => {
    if (isAuthenticated) {
      connect();

      // Listener per eventi WebSocket dal backend Rust
      const setupListeners = async () => {
        // Evento: connesso
        const unlistenConnected = await listen('ws-connected', () => {
          console.log('WebSocket connesso (evento Tauri)');
          setIsConnected(true);
          reconnectAttemptsRef.current = 0;
        });

        // Evento: messaggio ricevuto
        const unlistenMessage = await listen<string>('ws-message', (event) => {
          try {
            const data = JSON.parse(event.payload);
            
            // Gestione errori dal server
            if (data.error) {
              console.error('Errore dal server WebSocket:', data.error);
              errorCallbacksRef.current.forEach(callback => callback(data.error));
              return;
            }
            
            // Gestione segnali AddChat/RemoveChat
            if (data.AddChat !== undefined) {
              const chatId = data.AddChat;
              chatAddedCallbacksRef.current.forEach(callback => callback(chatId));
              return;
            }
            
            if (data.RemoveChat !== undefined) {
              const chatId = data.RemoveChat;
              chatRemovedCallbacksRef.current.forEach(callback => callback(chatId));
              return;
            }
            
            // Il backend invia i messaggi in batch (array)
            const messages: MessageDTO[] = Array.isArray(data) ? data : [data];
            
            // Elabora ogni messaggio nel batch
            messages.forEach((msg) => {
              if (msg.chat_id && msg.content) {
                const callbacks = chatCallbacksRef.current.get(msg.chat_id);
                if (callbacks) {
                  callbacks.forEach(callback => callback(msg));
                }
              }
            });
          } catch (error) {
            console.error('Errore parsing messaggio:', error);
          }
        });

        // Evento: disconnesso
        const unlistenDisconnected = await listen('ws-disconnected', () => {
          console.log('WebSocket disconnesso (evento Tauri)');
          setIsConnected(false);
          
          // Tentativo di riconnessione
          if (isAuthenticated && reconnectAttemptsRef.current < MAX_RECONNECT_ATTEMPTS) {
            reconnectAttemptsRef.current++;
            console.log(`Tentativo di riconnessione ${reconnectAttemptsRef.current}/${MAX_RECONNECT_ATTEMPTS}...`);
            reconnectTimeoutRef.current = setTimeout(connect, RECONNECT_DELAY) as unknown as number;
          }
        });

        // Evento: errore
        const unlistenError = await listen<string>('ws-error', (event) => {
          console.error('WebSocket errore (evento Tauri):', event.payload);
          setIsConnected(false);
          errorCallbacksRef.current.forEach(callback => callback(event.payload));
        });

        return () => {
          unlistenConnected();
          unlistenMessage();
          unlistenDisconnected();
          unlistenError();
        };
      };

      const cleanupPromise = setupListeners();

      return () => {
        cleanupPromise.then(cleanup => cleanup());
        if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current);
        }
        // Disconnetti il WebSocket
        invoke('disconnect_websocket').catch(console.error);
      };
    } else {
      setIsConnected(false);
      return () => {};
    }
  }, [isAuthenticated, connect]);

  const sendMessage = useCallback(async (message: MessageDTO) => {
    if (!isConnected) {
      console.error('WebSocket non connesso');
      errorCallbacksRef.current.forEach(callback => 
        callback('Connessione non disponibile. Riprova tra poco.')
      );
      return;
    }

    try {
      await invoke('send_websocket_message', {
        message: JSON.stringify(message),
      });
    } catch (error) {
      console.error('Errore invio messaggio:', error);
      errorCallbacksRef.current.forEach(callback => 
        callback('Errore durante l\'invio del messaggio')
      );
    }
  }, [isConnected]);

  const subscribeToChat = useCallback((chatId: number, callback: (message: MessageDTO) => void) => {
    if (!chatCallbacksRef.current.has(chatId)) {
      chatCallbacksRef.current.set(chatId, new Set());
    }
    chatCallbacksRef.current.get(chatId)!.add(callback);

    // Ritorna funzione per unsubscribe
    return () => {
      const callbacks = chatCallbacksRef.current.get(chatId);
      if (callbacks) {
        callbacks.delete(callback);
        if (callbacks.size === 0) {
          chatCallbacksRef.current.delete(chatId);
        }
      }
    };
  }, []);

  const onError = useCallback((callback: (error: string) => void) => {
    errorCallbacksRef.current.add(callback);

    // Ritorna funzione per unsubscribe
    return () => {
      errorCallbacksRef.current.delete(callback);
    };
  }, []);

  const onChatAdded = useCallback((callback: (chatId: number) => void) => {
    chatAddedCallbacksRef.current.add(callback);

    // Ritorna funzione per unsubscribe
    return () => {
      chatAddedCallbacksRef.current.delete(callback);
    };
  }, []);

  const onChatRemoved = useCallback((callback: (chatId: number) => void) => {
    chatRemovedCallbacksRef.current.add(callback);

    // Ritorna funzione per unsubscribe
    return () => {
      chatRemovedCallbacksRef.current.delete(callback);
    };
  }, []);

  const value: WebSocketContextType = {
    isConnected,
    sendMessage,
    subscribeToChat,
    onError,
    onChatAdded,
    onChatRemoved,
  };

  return <WebSocketContext.Provider value={value}>{children}</WebSocketContext.Provider>;
}
