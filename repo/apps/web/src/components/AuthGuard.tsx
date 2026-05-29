import { useEffect, type ReactNode } from 'react';

interface AuthGuardProps {
  children: ReactNode;
}

export function AuthGuard({ children }: AuthGuardProps) {
  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) {
      window.location.href = '/login';
    }
  }, []);

  const token = localStorage.getItem('onelink_token');
  if (!token) {
    return null;
  }

  return <>{children}</>;
}
