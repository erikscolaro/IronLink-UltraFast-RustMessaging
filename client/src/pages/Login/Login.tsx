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
  const [confpassword, setConfPassword] = useState('');
  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const { login, register } = useAuth();
  const navigate = useNavigate();

  function validatePassword(password: string) {
    const minLength = /.{8,}/;
    const hasUpper = /[A-Z]/;
    const hasLower = /[a-z]/;
    const hasNumber = /[0-9]/;
    const hasSpecial = /[!@#$%^&*(),.?":{}|<>]/;

    if (!minLength.test(password)) return "La password deve avere almeno 8 caratteri";
    if (!hasUpper.test(password)) return "La password deve contenere almeno una lettera maiuscola";
    if (!hasLower.test(password)) return "La password deve contenere almeno una lettera minuscola";
    if (!hasNumber.test(password)) return "La password deve contenere almeno un numero";
    if (!hasSpecial.test(password)) return "La password deve contenere almeno un carattere speciale";

  return null; // password valida
}

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setIsLoading(true);

    const result = validatePassword(password);
    if (result != null) {
      setError(result);
      setIsLoading(false);
      return;
    }

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
        <Col md={6} className={styles.leftPanel+ ' blur-1 shadow'}>
          <div className={styles.brandSection}>
            <h1 className={styles.brandTitle}>Ruggine</h1>
            <p className={styles.brandSubtitle}>Comunicazione forte come il ferro</p>
          </div>
        </Col>

        {/* Lato destro - Form */}
        <Col md={6} className={styles.rightPanel}>
          <div className={styles.formContainer+ ' blur-1 shadow'}>
          
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
                  className={styles.input}
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
                  className={styles.input}
                />
              </Form.Group>

              {
                isLogin?null:             
                <Form.Group className="mb-3" controlId="confirm-password">
                <Form.Label>Ripeti password</Form.Label>
                <Form.Control
                  type="password"
                  value={confpassword}
                  onChange={(e) => setConfPassword(e.target.value)}
                  placeholder="Inserisci password"
                  required
                  disabled={isLoading}
                  className={styles.input}
                />
              </Form.Group>
              }

              {error && (
                <Alert variant="danger">
                  {error}
                </Alert>
              )}

              <Button
                variant="danger"
                type="submit"
                disabled={isLoading||(!isLogin&&(password!=confpassword))||(password.length==0)||username.length==0}
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
                className='text-white'
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
