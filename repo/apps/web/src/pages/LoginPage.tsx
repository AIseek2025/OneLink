import { useEffect, useState } from 'react';
import { login, register } from '../api/client';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';

export function LoginPage() {
  const [mode, setMode] = useState<'login' | 'register'>('login');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');

  useEffect(() => {
    trackEvent(
      { event_name: mode === 'login' ? 'login.started' : 'registration.started', provider: 'email' },
      getAnalyticsContext({ screen: '/login' }),
    );
  }, [mode]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    try {
      if (mode === 'login') {
        const data = await login(email, password);
        const userId = data?.user?.user_id ?? '';
        trackEvent(
          { event_name: 'login.completed', user_id: userId, provider: 'email' },
          getAnalyticsContext({ screen: '/login', user_id: userId }),
        );
      } else {
        const data = await register(email, password, 'CN', 'zh');
        const userId = data?.user?.user_id ?? '';
        trackEvent(
          { event_name: 'registration.completed', user_id: userId, provider: 'email' },
          getAnalyticsContext({ screen: '/login', user_id: userId }),
        );
      }
      window.location.href = '/';
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Unknown error';
      setError(msg);
      trackEvent(
        { event_name: 'error.occurred', error_type: 'auth', error_message: msg },
        getAnalyticsContext({ screen: '/login' }),
      );
    }
  };

  return (
    <div style={{ maxWidth: 400, margin: '80px auto', padding: tokens.spacing.xl }}>
      <h1 style={{ color: tokens.color.brand.primary, fontSize: tokens.typography.fontSize['2xl'] }}>OneLink</h1>
      <h2 style={{ fontSize: tokens.typography.fontSize.xl, marginBottom: tokens.spacing.lg }}>
        {mode === 'login' ? '登录' : '注册'}
      </h2>
      {error && <p style={{ color: tokens.color.semantic.error, background: tokens.color.semantic['error-bg'], padding: tokens.spacing.md, borderRadius: tokens.borderRadius.md }}>{error}</p>}
      <form onSubmit={handleSubmit}>
        <div style={{ marginBottom: tokens.spacing.md }}>
          <input
            type="email"
            placeholder="邮箱"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
            style={{ width: '100%', padding: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm, border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.md, outline: 'none' }}
          />
        </div>
        <div style={{ marginBottom: tokens.spacing.md }}>
          <input
            type="password"
            placeholder="密码"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            style={{ width: '100%', padding: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm, border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.md, outline: 'none' }}
          />
        </div>
        <button type="submit" style={{
          width: '100%', padding: `${tokens.spacing.md} ${tokens.spacing.lg}`, fontSize: tokens.typography.fontSize.base,
          background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md,
          cursor: 'pointer', fontWeight: tokens.typography.fontWeight.semibold,
        }}>
          {mode === 'login' ? '登录' : '注册'}
        </button>
      </form>
      <p style={{ marginTop: tokens.spacing.md, textAlign: 'center', color: tokens.color.neutral['text-secondary'] }}>
        {mode === 'login' ? (
          <>没有账号？<a href="#" onClick={() => setMode('register')} style={{ color: tokens.color.brand.primary }}>注册</a></>
        ) : (
          <>已有账号？<a href="#" onClick={() => setMode('login')} style={{ color: tokens.color.brand.primary }}>登录</a></>
        )}
      </p>
    </div>
  );
}
