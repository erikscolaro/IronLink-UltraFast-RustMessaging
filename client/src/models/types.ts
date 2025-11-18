// Tipi che corrispondono ai DTO del backend

export enum ChatType {
  Private = "Private",
  Group = "Group"
}

export enum MessageType {
  UserMessage = "UserMessage",
  SystemMessage = "SystemMessage"
}

export enum UserRole {
  Owner = "Owner",
  Admin = "Admin",
  Member = "Member"
}

export interface UserDTO {
  id?: number; // Campo dal backend
  user_id?: number; // Retrocompatibilità
  username?: string;
}

// Helper per ottenere l'ID utente (gestisce sia id che user_id)
export function getUserId(user: UserDTO): number {
  return user.id || user.user_id || 0;
}

export interface ChatDTO {
  chat_id: number;
  title?: string;
  description?: string;
  chat_type: ChatType;
  created_at?: string;
  user_list?: number[]; // Lista ID utenti per chat private/gruppo
}

export interface MessageDTO {
  message_id?: number;
  chat_id?: number;
  sender_id?: number;
  content?: string;
  message_type?: MessageType;
  created_at?: string;
}

export interface CreateMessageDTO {
  chat_id: number;
  sender_id: number;
  content: string;
  message_type: MessageType;
  created_at: string;
}

export interface InvitationDTO {
  invitation_id: number;
  chat_id: number;
  inviter_id: number;
  invitee_id: number;
  created_at: string;
}

export interface EnrichedInvitationDTO {
  invite_id: number;
  state: string;
  created_at: string;
  inviter?: UserDTO;
  chat?: ChatDTO;
}

export interface UserChatMetadataDTO {
  user_id: number;
  chat_id: number;
  user_role?: UserRole; // Opzionale perché viene dal backend come Option
  member_since?: string; // Opzionale perché viene dal backend come Option
  username?: string; // Nome utente (opzionale)
}

// Tipi locali per lo stato dell'applicazione
export interface PendingMessage extends MessageDTO {
  localId: string;
  status: 'pending' | 'sent' | 'error';
}

export interface ChatWithMembers extends ChatDTO {
  members?: UserDTO[];
  lastMessage?: MessageDTO;
}
