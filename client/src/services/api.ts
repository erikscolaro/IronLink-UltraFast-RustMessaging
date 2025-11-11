// API Service - Gestisce tutte le chiamate HTTP al backend
import { ChatDTO, UserDTO, MessageDTO, EnrichedInvitationDTO, UserChatMetadataDTO, ChatType, UserRole } from '../models/types';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:8080';

// Interfaccia per gli errori del backend
interface BackendError {
  error: string;
  details?: string;
}

// Utility per gestire le risposte HTTP
async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    let errorMessage = `HTTP error! status: ${response.status}`;
    
    try {
      // Prova a parsare l'errore come JSON
      const errorData: BackendError = await response.json();
      
      // Stampa l'errore completo nella console per debug
      console.error('Backend Error:', {
        status: response.status,
        error: errorData.error,
        details: errorData.details,
      });
      
      // Usa solo il campo "error" per mostrare all'utente
      errorMessage = errorData.error || errorMessage;
    } catch (e) {
      console.error('Could not parse error response:', e);
    }
    
    throw new Error(errorMessage);
  }
  
  // Se la risposta è vuota (204 No Content o simili)
  if (response.status === 204 || response.headers.get('content-length') === '0') {
    return {} as T;
  }
  
  return response.json();
}

// Ottiene il token dal localStorage
function getAuthToken(): string | null {
  return localStorage.getItem('token');
}

// Headers comuni per richieste autenticate
function getAuthHeaders(): HeadersInit {
  const token = getAuthToken();
  return {
    'Content-Type': 'application/json',
    ...(token && { 'Authorization': `Bearer ${token}` })
  };
}

// ==================== AUTH ====================

export interface LoginRequest {
  username: string;
  password: string;
}

export interface RegisterRequest {
  username: string;
  password: string;
}

export async function login(credentials: LoginRequest): Promise<string> {
  const response = await fetch(`${API_BASE_URL}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(credentials),
  });
  
  if (!response.ok) {
    let errorMessage = 'Login fallito. Verifica username e password.';
    
    try {
      const errorData: BackendError = await response.json();
      console.error('Login Error:', {
        status: response.status,
        error: errorData.error,
        details: errorData.details,
      });
      errorMessage = errorData.error || errorMessage;
    } catch (e) {
      console.error('Could not parse error response:', e);
    }
    
    throw new Error(errorMessage);
  }
  
  // Login riuscito - il token è nell'header Authorization
  const authHeader = response.headers.get('Authorization');
  if (!authHeader) {
    console.error('Response headers:', Object.fromEntries(response.headers.entries()));
    throw new Error('Token non ricevuto dal server');
  }
  
  // Estrae il token da "Bearer <token>"
  const token = authHeader.replace('Bearer ', '');
  localStorage.setItem('token', token);
  
  return token;
}

export async function register(credentials: RegisterRequest): Promise<UserDTO> {
  const response = await fetch(`${API_BASE_URL}/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(credentials),
  });
  
  return handleResponse<UserDTO>(response);
}

export function logout(): void {
  localStorage.removeItem('token');
}

// ==================== USERS ====================

export async function getCurrentUser(): Promise<UserDTO> {
  const token = getAuthToken();
  if (!token) {
    throw new Error('No token found');
  }
  
  // Usa la nuova rotta GET /users/me per ottenere i dati dell'utente corrente
  const response = await fetch(`${API_BASE_URL}/users/me`, {
    headers: getAuthHeaders(),
  });
  
  return handleResponse<UserDTO>(response);
}

export async function getUserById(userId: number): Promise<UserDTO> {
  const response = await fetch(`${API_BASE_URL}/users/${userId}`, {
    headers: getAuthHeaders(),
  });
  
  return handleResponse<UserDTO>(response);
}

export async function searchUserByUsername(username: string): Promise<UserDTO[]> {
  const response = await fetch(`${API_BASE_URL}/users?search=${encodeURIComponent(username)}`, {
    headers: getAuthHeaders(),
  });
  
  return handleResponse<UserDTO[]>(response);
}

export async function deleteMyAccount(): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/users/me`, {
    method: 'DELETE',
    headers: getAuthHeaders(),
  });
  
  await handleResponse<void>(response);
  logout();
}

// ==================== CHATS ====================

export async function listChats(): Promise<ChatDTO[]> {
  const response = await fetch(`${API_BASE_URL}/chats`, {
    headers: getAuthHeaders(),
  });
  
  return handleResponse<ChatDTO[]>(response);
}

export interface CreateChatRequest {
  title?: string;
  description?: string;
  chat_type: ChatType;
  user_list?: number[]; // Solo per chat private
}

export async function createChat(chatData: CreateChatRequest): Promise<ChatDTO> {
  const response = await fetch(`${API_BASE_URL}/chats`, {
    method: 'POST',
    headers: getAuthHeaders(),
    body: JSON.stringify(chatData),
  });
  
  return handleResponse<ChatDTO>(response);
}

export async function getChatMessages(chatId: number, beforeDate?: string): Promise<MessageDTO[]> {
  let url = `${API_BASE_URL}/chats/${chatId}/messages`;
  
  // Il backend accetta un parametro "before_date" per la paginazione
  // Se fornito, restituisce i 50 messaggi precedenti a quella data
  // Se non fornito, restituisce gli ultimi 50 messaggi
  if (beforeDate) {
    url += `?before_date=${encodeURIComponent(beforeDate)}`;
  }
  
  const response = await fetch(url, {
    headers: getAuthHeaders(),
  });
  
  return handleResponse<MessageDTO[]>(response);
}

export async function listChatMembers(chatId: number): Promise<UserChatMetadataDTO[]> {
  const response = await fetch(`${API_BASE_URL}/chats/${chatId}/members`, {
    headers: getAuthHeaders(),
  });
  
  return handleResponse<UserChatMetadataDTO[]>(response);
}

export async function inviteToChat(chatId: number, userId: number): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/chats/${chatId}/invite/${userId}`, {
    method: 'POST',
    headers: getAuthHeaders(),
  });
  
  await handleResponse<void>(response);
}

export async function updateMemberRole(chatId: number, userId: number, role: UserRole): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/chats/${chatId}/members/${userId}/role`, {
    method: 'PATCH',
    headers: getAuthHeaders(),
    body: JSON.stringify(role),
  });
  
  await handleResponse<void>(response);
}

export async function removeMember(chatId: number, userId: number): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/chats/${chatId}/members/${userId}`, {
    method: 'DELETE',
    headers: getAuthHeaders(),
  });
  
  await handleResponse<void>(response);
}

export async function leaveChat(chatId: number): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/chats/${chatId}/leave`, {
    method: 'POST',
    headers: getAuthHeaders(),
  });
  
  await handleResponse<void>(response);
}

export async function transferOwnership(chatId: number, newOwnerId: number): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/chats/${chatId}/transfer_ownership`, {
    method: 'PATCH',
    headers: getAuthHeaders(),
    body: JSON.stringify({ new_owner_id: newOwnerId }),
  });
  
  await handleResponse<void>(response);
}

export async function cleanChat(chatId: number): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/chats/${chatId}/clean`, {
    method: 'POST',
    headers: getAuthHeaders(),
  });
  
  await handleResponse<void>(response);
}

// ==================== INVITATIONS ====================

export async function listPendingInvitations(): Promise<EnrichedInvitationDTO[]> {
  const response = await fetch(`${API_BASE_URL}/invitations/pending`, {
    headers: getAuthHeaders(),
  });
  
  return handleResponse<EnrichedInvitationDTO[]>(response);
}

export async function respondToInvitation(inviteId: number, action: 'accept' | 'decline'): Promise<void> {
  // Il backend si aspetta 'reject' invece di 'decline'
  const backendAction = action === 'decline' ? 'reject' : action;
  const response = await fetch(`${API_BASE_URL}/invitations/${inviteId}/${backendAction}`, {
    method: 'POST',
    headers: getAuthHeaders(),
  });
  
  await handleResponse<void>(response);
}
