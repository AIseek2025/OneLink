import { useEffect, useState } from 'react';
import { fetchHome } from '../api/client';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';
import { OlNavBar } from '../components/OlNavBar';

interface HomeData {
  user: { user_id: string; status: string; primary_region: string; primary_language: string };
  profile: { display_name: string; city_level_location: string } | null;
  completion: { completion_rate: number; missing_dimensions: string[] } | null;
}

export function HomePage() {
  const [data, setData] = useState<HomeData | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    fetchHome()
      .then((d) => {
        setData(d);
        trackEvent({ event_name: 'page.view', page_path: '/' }, getAnalyticsContext({ screen: '/', user_id: d.user?.user_id ?? null }));
      })
      .catch((e) => {
        setError(e.message);
        trackEvent({ event_name: 'error.occurred', error_type: 'network', error_message: e.message }, getAnalyticsContext({ screen: '/' }));
      });
  }, []);

  const navItems = [
    { label: '首页', href: '/', active: true },
    { label: '聊天', href: '/chat' },
    { label: '画像', href: '/profile' },
    { label: '找人', href: '/find' },
    { label: '问卷', href: '/questionnaire' },
  ];

  const handleLogout = () => {
    localStorage.removeItem('onelink_token');
    window.location.href = '/login';
  };

  if (error) return <div style={{ padding: tokens.spacing.xl, color: tokens.color.semantic.error }}>Error: {error}</div>;
  if (!data) return <div style={{ padding: tokens.spacing.xl, color: tokens.color.neutral['text-secondary'] }}>Loading...</div>;

  return (
    <div>
      <OlNavBar items={navItems} onLogout={handleLogout} />
      <div style={{ maxWidth: 640, margin: '0 auto', padding: tokens.spacing.xl }}>
        <h1 style={{ color: tokens.color.brand.primary, fontSize: tokens.typography.fontSize['2xl'] }}>OneLink</h1>
        <div style={{ marginBottom: tokens.spacing.lg }}>
          <p style={{ color: tokens.color.neutral['text-primary'], fontWeight: tokens.typography.fontWeight.medium }}>
            欢迎, {data.profile?.display_name || data.user.user_id}
          </p>
          <p style={{ color: tokens.color.neutral['text-secondary'], fontSize: tokens.typography.fontSize.sm }}>
            {data.user.primary_region} · {data.user.primary_language}
          </p>
          {data.completion && (
            <p style={{ color: tokens.color.neutral['text-secondary'], fontSize: tokens.typography.fontSize.sm }}>
              画像完成度: {Math.round(data.completion.completion_rate * 100)}%
            </p>
          )}
        </div>
        <div style={{ display: 'flex', gap: tokens.spacing.md, flexWrap: 'wrap' }}>
          <a href="/chat" style={{ padding: `${tokens.spacing.md} ${tokens.spacing.lg}`, background: tokens.color.brand.primary, color: '#FFFFFF', borderRadius: tokens.borderRadius.md, textDecoration: 'none', fontWeight: tokens.typography.fontWeight.semibold }}>Lumi Chat</a>
          <a href="/profile" style={{ padding: `${tokens.spacing.md} ${tokens.spacing.lg}`, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.md, textDecoration: 'none', color: tokens.color.neutral['text-primary'] }}>画像</a>
          <a href="/find" style={{ padding: `${tokens.spacing.md} ${tokens.spacing.lg}`, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.md, textDecoration: 'none', color: tokens.color.neutral['text-primary'] }}>找人</a>
          <a href="/questionnaire" style={{ padding: `${tokens.spacing.md} ${tokens.spacing.lg}`, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.md, textDecoration: 'none', color: tokens.color.neutral['text-primary'] }}>问卷</a>
        </div>
      </div>
    </div>
  );
}
