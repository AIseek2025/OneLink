import { useState, useCallback } from 'react';
import { login, register } from '../api/client';
import { trackEvent, getAnalyticsContext } from '../analytics';

interface AuthState {
  token: string | null;
  userId: string | null;
  isLoading: boolean;
  error: string | null;
}

export function useAuth() {
  const [state, setState] = useState<AuthState>(() => {
    const token = localStorage.getItem('onelink_token');
    let userId: string | null = null;
    if (token) {
      try {
        const payload = JSON.parse(atob(token.split('.')[0] || ''));
        userId = payload?.user_id ?? null;
      } catch {
        // ignore
      }
    }
    return { token, userId, isLoading: false, error: null };
  });

  const handleLogin = useCallback(async (email: string, password: string) => {
    setState((s) => ({ ...s, isLoading: true, error: null }));
    try {
      const data = await login(email, password);
      const userId = data?.user?.user_id ?? null;
      const token = data?.session?.token ?? null;
      if (token) localStorage.setItem('onelink_token', token);
      setState({ token, userId, isLoading: false, error: null });
      trackEvent(
        { event_name: 'login_success', user_id: userId ?? '', provider: 'email' },
        getAnalyticsContext({ screen: '/login', user_id: userId }),
      );
      return true;
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Login failed';
      setState((s) => ({ ...s, isLoading: false, error: msg }));
      trackEvent(
        { event_name: 'error.occurred', error_type: 'auth', error_message: msg },
        getAnalyticsContext({ screen: '/login' }),
      );
      return false;
    }
  }, []);

  const handleRegister = useCallback(async (email: string, password: string, region: string, language: string) => {
    setState((s) => ({ ...s, isLoading: true, error: null }));
    try {
      const data = await register(email, password, region, language);
      const userId = data?.user?.user_id ?? null;
      const token = data?.session?.token ?? null;
      if (token) localStorage.setItem('onelink_token', token);
      setState({ token, userId, isLoading: false, error: null });
      trackEvent(
        { event_name: 'register_success', user_id: userId ?? '', provider: 'email' },
        getAnalyticsContext({ screen: '/login', user_id: userId }),
      );
      return true;
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Registration failed';
      setState((s) => ({ ...s, isLoading: false, error: msg }));
      trackEvent(
        { event_name: 'error.occurred', error_type: 'auth', error_message: msg },
        getAnalyticsContext({ screen: '/login' }),
      );
      return false;
    }
  }, []);

  const logout = useCallback(() => {
    localStorage.removeItem('onelink_token');
    setState({ token: null, userId: null, isLoading: false, error: null });
  }, []);

  const clearError = useCallback(() => {
    setState((s) => ({ ...s, error: null }));
  }, []);

  return {
    token: state.token,
    userId: state.userId,
    isAuthenticated: !!state.token,
    isLoading: state.isLoading,
    error: state.error,
    login: handleLogin,
    register: handleRegister,
    logout,
    clearError,
  };
}