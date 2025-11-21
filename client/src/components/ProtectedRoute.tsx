// ProtectedRoute - Componente per proteggere le route autenticate
import { Navigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { Spinner, Container } from 'react-bootstrap';

interface ProtectedRouteProps {
  children: React.ReactNode;
}

export default function ProtectedRoute({ children }: ProtectedRouteProps) {
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) {
    return (
      <Container fluid className="d-flex align-items-center justify-content-center vh-100">
        <div className="text-center">
          <Spinner animation="border" variant="secondary" />
          <p className="mt-2">Verifica autenticazione...</p>
        </div>
      </Container>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return <>{children}</>;
}
