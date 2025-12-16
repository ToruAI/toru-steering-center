import React, { createContext, useContext, useEffect, useState, useCallback } from 'react';
import { api, setAuthErrorHandler } from '../lib/api';
import { useNavigate } from 'react-router-dom';

export interface LoginResult {
  success: boolean;
  error?: string;
  lockedUntil?: number;
}

interface AuthContextType {
  user: {
    id: string | null;
    username: string;
    display_name: string | null;
    role: 'admin' | 'client';
  } | null;
  loading: boolean;
  login: (username: string, password: string) => Promise<LoginResult>;
  logout: () => Promise<void>;
  isAdmin: boolean;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<AuthContextType['user']>(null);
  const [loading, setLoading] = useState(true);
  const navigate = useNavigate();

  // Handle session expiry from API calls
  const handleAuthError = useCallback(() => {
    setUser(null);
    navigate('/login', { state: { message: 'Session expired. Please log in again.' } });
  }, [navigate]);

  useEffect(() => {
    // Register global auth error handler
    setAuthErrorHandler(handleAuthError);
    checkAuth();
  }, [handleAuthError]);

  const checkAuth = async () => {
    try {
      const { authenticated, user } = await api.me();
      if (authenticated && user) {
        setUser(user);
      } else {
        setUser(null);
      }
    } catch (err) {
      console.error('Auth check failed:', err);
      setUser(null);
    } finally {
      setLoading(false);
    }
  };

  const login = async (username: string, password: string): Promise<LoginResult> => {
    try {
      const res = await api.login(username, password);
      if (res.success && res.user) {
        setUser(res.user);
        navigate('/');
        return { success: true };
      }
      // Return lockout info if present
      if (res.locked_until) {
        return { success: false, error: res.error || 'Too many attempts', lockedUntil: res.locked_until };
      }
      return { success: false, error: res.error || 'Login failed' };
    } catch (err) {
      console.error('Login error:', err);
      return { success: false, error: 'An unexpected error occurred' };
    }
  };

  const logout = async () => {
    try {
      await api.logout();
      setUser(null);
      navigate('/login');
    } catch (err) {
      console.error('Logout error:', err);
    }
  };

  return (
    <AuthContext.Provider 
      value={{ 
        user, 
        loading, 
        login, 
        logout,
        isAdmin: user?.role === 'admin' 
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
