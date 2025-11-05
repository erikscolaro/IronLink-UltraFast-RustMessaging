// Sidebar.tsx
import styles from "./Sidebar.module.css";
import { useState } from "react";
import { Chat, ChatType, Invite } from "../../models/Chat";
import SideChatCard from "../SideChatCard/SideChatCard";

interface SidebarProps {
  items: (Chat | Invite)[];
  setItem: (items: (Chat | Invite)[]) => void;
  selectedItem: string|null;
  selectItem: (key: string|null) => void;
}

export default function Sidebar({
  items,
  setItem,
  selectedItem,
  selectItem,
}: SidebarProps) {
  const [filter, setFilter] = useState<"all" | "group" | "private" | "invite">(
    "all"
  );

  const handleClick = (type: "group" | "private" | "invite") => {
    setFilter(filter === type ? "all" : type); // toggle
  };

  const filteredItems = items.filter((item) => {
    if (filter === "group")
      return "type" in item && item.type === ChatType.Group;
    if (filter === "private")
      return "type" in item && item.type === ChatType.Private;
    if (filter === "invite") return "from" in item;
    return true;
  });

  return (
    <div className={styles.sidebar}>
      <div className={styles.chatTitle}>Chats</div>
      <div className="d-flex gap-2">
        <button onClick={() => handleClick("group")} className={styles.sideButton}>Gruppi</button>
        <button onClick={() => handleClick("private")} className={styles.sideButton} >Private</button>
        <button onClick={() => handleClick("invite")} className={styles.sideButton} >Inviti</button>
      </div>
      <div className="d-flex flex-column gap-2 ">
        {filteredItems.map((item) => {
          const key = "type" in item ? `chat-${item.id}` : `invite-${item.id}`;
          return "type" in item? <SideChatCard chat={item} setItem={selectItem}/>: <div key={key}>{item.id}</div>;
        })}
      </div>
    </div>
  );
}
