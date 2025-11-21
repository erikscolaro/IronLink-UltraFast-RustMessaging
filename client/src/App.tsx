// App.tsx - Configurazione router principale dell'applicazione
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider } from './context/AuthContext';
import { WebSocketProvider } from './context/WebSocketContext';
import ProtectedRoute from './components/ProtectedRoute';
import Login from './pages/Login/Login';
import Home from './pages/Home/Home';
import 'bootstrap/dist/css/bootstrap.min.css';
import 'bootstrap-icons/font/bootstrap-icons.css';
import './assets/common.css';

function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <WebSocketProvider>
          <Routes>
            {/* Route pubblica - Login */}
            <Route path="/login" element={<Login />} />
            
            {/* Route protetta - Home */}
            <Route
              path="/home"
              element={
                <ProtectedRoute>
                  <Home />
                </ProtectedRoute>
              }
            />
            
            {/* Redirect root alla home */}
            <Route path="/" element={<Navigate to="/home" replace />} />
            
            {/* 404 - Redirect alla home */}
            <Route path="*" element={<Navigate to="/home" replace />} />
          </Routes>
        </WebSocketProvider>
      </AuthProvider>
    </BrowserRouter>
  );
}

export default App;