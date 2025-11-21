// AuthContext - Gestisce lo stato di autenticazione globale
import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { UserDTO } from '../models/types';
import * as api from '../services/api';

interface AuthContextType {
  user: UserDTO | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (username: string, password: string) => Promise<void>;
  register: (username: string, password: string) => Promise<void>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth deve essere usato all\'interno di AuthProvider');
  }
  return context;
}

interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<UserDTO | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Verifica se l'utente è già autenticato all'avvio
  useEffect(() => {
    const checkAuth = async () => {
      const token = localStorage.getItem('token');
      if (token) {
        try {
          // Tenta di ottenere i dati dell'utente corrente
          const userData = await api.getCurrentUser();
          setUser(userData);
        } catch (error) {
          console.error('Token non valido:', error);
          localStorage.removeItem('token');
        }
      }
      setIsLoading(false);
    };

    checkAuth();
  }, []);

  const login = async (username: string, password: string) => {
    try {
      await api.login({ username, password });
      // Dopo il login, ottieni i dati dell'utente
      const userData = await api.getCurrentUser();
      setUser(userData);
    } catch (error) {
      console.error('Errore durante il login:', error);
      throw error;
    }
  };

  const register = async (username: string, password: string) => {
    try {
      await api.register({ username, password });
      // Dopo la registrazione, esegui automaticamente il login
      await api.login({ username, password });
      const fullUserData = await api.getCurrentUser();
      setUser(fullUserData);
    } catch (error) {
      console.error('Errore durante la registrazione:', error);
      throw error;
    }
  };

  const logout = () => {
    api.logout();
    setUser(null);
  };

  const value: AuthContextType = {
    user,
    isAuthenticated: !!user,
    isLoading,
    login,
    register,
    logout,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
