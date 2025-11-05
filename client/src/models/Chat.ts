export enum ChatType {
  Private,
  Group
}

export interface Chat {
  id: number;
  name?: string;
  description?: string;
  type: ChatType
  members: User[];
  messages: Message[];
}


export interface User {
  id: number;
  username: string;
  member_since: string;
}

export interface Message {
  id: number;
  content: string;
  timestamp: Date;
}

export interface Invite {
  id: number;
  content: string;
  from: number
}

export interface Items {
  content: (Chat | Invite)[];
}