import { useState, useCallback } from 'react';
import { setAuthToken } from '../services/bffClient';
import { persistAuthTokens, clearAuthTokens, getPersistedAccessToken } from '../services/config';
import { bffClient } from '../services/bffClient';
import type { AuthResponse, User } from '../types';

type AuthState = 'unknown' | 'authenticated' | 'unauthenticated';

export function useAuth() {
  const [authState, setAuthState] = useState<AuthState>('unknown');
  const [user, setUser] = useState<User | null>(null);

  const restoreSession = useCallback(async () => {
    const token = await getPersistedAccessToken();
    if (!token) {
      setAuthState('unauthenticated');
      return;
    }
    setAuthToken(token);
    try {
      const resp = await bffClient.get<{ user: User }>('/auth/me');
      setUser(resp.user);
      setAuthState('authenticated');
    } catch {
      setAuthToken(null);
      await clearAuthTokens();
      setAuthState('unauthenticated');
    }
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    const resp = await bffClient.post<AuthResponse>('/auth/login', { email, password });
    setAuthToken(resp.access_token);
    await persistAuthTokens(resp.access_token, resp.refresh_token);
    setAuthState('authenticated');
  }, []);

  const register = useCallback(async (email: string, password: string, nickname: string) => {
    const resp = await bffClient.post<AuthResponse>('/auth/register', { email, password, nickname });
    setAuthToken(resp.access_token);
    await persistAuthTokens(resp.access_token, resp.refresh_token);
    if (resp.flow_state !== 'requires_verification') {
      setAuthState('authenticated');
    }
    return resp.flow_state;
  }, []);

  const logout = useCallback(async () => {
    setAuthToken(null);
    await clearAuthTokens();
    setUser(null);
    setAuthState('unauthenticated');
  }, []);

  return { authState, user, restoreSession, login, register, logout };
}
