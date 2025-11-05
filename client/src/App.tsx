import { Button, Card, Col, Container, Row } from "react-bootstrap";
import Sidebar from "./components/Sidebar/Sidebar";
import { useState } from "react";
import { User, Chat, ChatType, Invite} from "./models/Chat";


const users: User[] = [
  { id: 1, username: "erik", member_since: "2022-01-01" },
  { id: 2, username: "lucas", member_since: "2023-04-15" },
  { id: 3, username: "mario", member_since: "2024-09-09" }
];

const chats: Chat[] = [
  {
    id: 1,
    name: "Università",
    description: "Gruppo di studio",
    type: ChatType.Group,
    members: [users[0], users[1], users[2]],
    messages: [
      { id: 1, content: "Ciao a tutti", timestamp: new Date() },
      { id: 2, content: "Domani esame?", timestamp: new Date() }
    ]
  },
  {
    id: 2,
    name: "Privata con Lucas",
    type: ChatType.Private,
    members: [users[0], users[1]],
    messages: [
      { id: 1, content: "Pronto per il progetto?", timestamp: new Date() }
    ]
  }
];

const invites: Invite[] = [
  { id: 1, content: "Invito al gruppo Università", from: 2 },
  { id: 2, content: "Richiesta chat privata", from: 3 }
];


function App() {

  const [items, setItems] = useState<(Chat | Invite)[]>([...chats, ...invites]);
  const [selected, setSelected] = useState<(string | null)>(null);

  return (
    <Container fluid className=" fill-window p-4 m-0">
      <Row className="fill">
        <Col md={3} >
          <Sidebar items={items} setItem={setItems} selectedItem={selected} selectItem={setSelected}/>
        </Col>
      
      </Row>
    </Container>
  );
}

export default App;