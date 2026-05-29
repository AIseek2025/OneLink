export function getStoredToken(): string | null {
  return localStorage.getItem('onelink_token');
}

export function getStoredUserId(): string | null {
  return localStorage.getItem('onelink_user_id');
}

export function persistSession(sessionToken: string | null | undefined, userId?: string | null) {
  if (sessionToken) {
    localStorage.setItem('onelink_token', sessionToken);
  }
  if (userId) {
    localStorage.setItem('onelink_user_id', userId);
  }
}

export function clearSession() {
  localStorage.removeItem('onelink_token');
  localStorage.removeItem('onelink_user_id');
}

export function extractResponseUserId(data: unknown): string | null {
  if (!data || typeof data !== 'object') return null;
  const obj = data as Record<string, unknown>;
  if (typeof obj.user_id === 'string') return obj.user_id;
  const nestedUser = obj.user;
  if (nestedUser && typeof nestedUser === 'object') {
    const nestedUserId = (nestedUser as Record<string, unknown>).user_id;
    if (typeof nestedUserId === 'string') return nestedUserId;
  }
  return null;
}
