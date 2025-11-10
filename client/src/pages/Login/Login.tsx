// Login Page - Gestisce login e registrazione utenti
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../../context/AuthContext';
import { Container, Row, Col, Form, Button, Alert } from 'react-bootstrap';
import styles from './Login.module.css';

export default function Login() {
  const [isLogin, setIsLogin] = useState(true);
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const { login, register } = useAuth();
  const navigate = useNavigate();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setIsLoading(true);

    try {
      if (isLogin) {
        await login(username, password);
      } else {
        await register(username, password);
      }
      navigate('/home');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Si è verificato un errore');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Container fluid className={styles.loginContainer}>
      <Row className="h-100">
        {/* Lato sinistro - Frase ad effetto */}
        <Col md={6} className={styles.leftPanel}>
          <div className={styles.brandSection}>
            <h1 className={styles.brandTitle}>Ruggine</h1>
            <p className={styles.brandSubtitle}>Comunicazione forte come il ferro</p>
            <div className={styles.brandDescription}>
              <p>Connessioni solide, messaggi istantanei.</p>
              <p>Temprato dal tempo, plasmato per durare.</p>
            </div>
          </div>
        </Col>

        {/* Lato destro - Form */}
        <Col md={6} className={styles.rightPanel}>
          <div className={styles.formContainer}>
            <h2 className={styles.formTitle}>
              {isLogin ? 'Accedi' : 'Registrati'}
            </h2>

            <Form onSubmit={handleSubmit}>
              <Form.Group className="mb-3" controlId="username">
                <Form.Label>Username</Form.Label>
                <Form.Control
                  type="text"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder="Inserisci username"
                  required
                  disabled={isLoading}
                />
              </Form.Group>

              <Form.Group className="mb-3" controlId="password">
                <Form.Label>Password</Form.Label>
                <Form.Control
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="Inserisci password"
                  required
                  disabled={isLoading}
                />
              </Form.Group>

              {error && (
                <Alert variant="danger">
                  {error}
                </Alert>
              )}

              <Button
                variant="danger"
                type="submit"
                disabled={isLoading}
                className="w-100 mb-3"
              >
                {isLoading ? 'Caricamento...' : (isLogin ? 'Accedi' : 'Registrati')}
              </Button>
            </Form>

            <div className="text-center">
              <Button
                variant="link"
                onClick={() => {
                  setIsLogin(!isLogin);
                  setError('');
                }}
                disabled={isLoading}
              >
                {isLogin ? 'Non hai un account? Registrati' : 'Hai già un account? Accedi'}
              </Button>
            </div>
          </div>
        </Col>
      </Row>
    </Container>
  );
}
