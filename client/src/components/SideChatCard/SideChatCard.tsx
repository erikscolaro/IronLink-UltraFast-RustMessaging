import React from "react";
import { Chat } from "../../models/Chat";
import style from "./SideChatCard.module.css";
import "bootstrap-icons/font/bootstrap-icons.css";

interface SideChatCardProps {
  chat: Chat;
  setItem: (key: string | null) => void;
}

export default function SideChatCard({ chat, setItem }: SideChatCardProps) {
  return (
    <div className={style.card2}>
      <i className={`bi bi-person ${style.icon}`}></i>
      <div className={style.cardText}>
        <div className={style.chatName}>{chat.name ?? "none"}</div>
        <div>{chat.description}</div>
      </div>
    </div>
  );
}
