import { useEffect, useState } from 'react';
import { fetchProfile, patchProfile } from '../api/client';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';

interface ProfileData {
  user: { user_id: string; status: string; primary_region: string };
  profile: {
    display_name: string;
    avatar_url: string;
    city_level_location: string;
    languages: string[];
    is_searchable: boolean;
    allow_discovery: boolean;
    facts: Array<{ fact_type: string; value: string; confidence: number }>;
    traits: {
      interest_tags: string[];
      connection_goal_tags: string[];
      location_label: string | null;
      communication_preferences: string[];
    };
  } | null;
  completion: { completion_rate: number; missing_dimensions: string[] } | null;
}

export function ProfilePage() {
  const [data, setData] = useState<ProfileData | null>(null);
  const [editing, setEditing] = useState(false);
  const [displayName, setDisplayName] = useState('');
  const [cityLocation, setCityLocation] = useState('');
  const [error, setError] = useState('');

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    const payload = JSON.parse(atob(token.split('.')[0] || ''));
    const userId = payload?.user_id || 'me';
    fetchProfile(userId)
      .then((d) => {
        setData(d);
        const uid = d.user?.user_id ?? null;
        trackEvent({ event_name: 'page.view', page_path: '/profile' }, getAnalyticsContext({ screen: '/profile', user_id: uid }));
        if (d.completion) {
          trackEvent(
            { event_name: 'profile.confirmation.viewed', user_id: uid ?? '', completion_rate: d.completion.completion_rate, missing_dimensions: d.completion.missing_dimensions },
            getAnalyticsContext({ screen: '/profile', user_id: uid }),
          );
        }
      })
      .catch((e) => {
        setError(e.message);
        trackEvent({ event_name: 'error.occurred', error_type: 'network', error_message: e.message }, getAnalyticsContext({ screen: '/profile' }));
      });
  }, []);

  const handleSave = async () => {
    try {
      const fields: Record<string, unknown> = {};
      if (displayName !== (data?.profile?.display_name ?? '')) fields.display_name = displayName;
      if (cityLocation !== (data?.profile?.city_level_location ?? '')) fields.city_level_location = cityLocation;
      if (Object.keys(fields).length > 0) {
        await patchProfile(fields);
        trackEvent(
          { event_name: 'profile.fact.confirmed', user_id: data?.user?.user_id ?? '', fact_type: 'profile_edit', fact_value: JSON.stringify(fields) },
          getAnalyticsContext({ screen: '/profile', user_id: data?.user?.user_id ?? null }),
        );
      }
      setEditing(false);
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Save failed';
      setError(msg);
      trackEvent({ event_name: 'error.occurred', error_type: 'network', error_message: msg }, getAnalyticsContext({ screen: '/profile' }));
    }
  };

  if (error) return <div style={{ padding: tokens.spacing.xl, color: tokens.color.semantic.error }}>Error: {error}</div>;
  if (!data) return <div style={{ padding: tokens.spacing.xl, color: tokens.color.neutral['text-secondary'] }}>Loading...</div>;

  const profile = data.profile;
  const completion = data.completion;

  return (
    <div style={{ maxWidth: 640, margin: '0 auto', padding: tokens.spacing.xl }}>
      <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'] }}>个人画像</h2>
      {completion && (
        <div style={{ marginBottom: tokens.spacing.lg, padding: tokens.spacing.md, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg }}>
          <p style={{ color: tokens.color.neutral['text-primary'], fontWeight: tokens.typography.fontWeight.medium }}>
            完成度: {Math.round(completion.completion_rate * 100)}%
          </p>
          {completion.missing_dimensions.length > 0 && (
            <p style={{ color: tokens.color.neutral['text-secondary'], fontSize: tokens.typography.fontSize.sm }}>
              待完善: {completion.missing_dimensions.join(', ')}
            </p>
          )}
        </div>
      )}
      {editing ? (
        <div>
          <label style={{ display: 'block', marginBottom: tokens.spacing.md, color: tokens.color.neutral['text-primary'] }}>
            显示名称: <input value={displayName} onChange={(e) => setDisplayName(e.target.value)} style={{ padding: tokens.spacing.sm, border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.sm }} />
          </label>
          <label style={{ display: 'block', marginBottom: tokens.spacing.md, color: tokens.color.neutral['text-primary'] }}>
            城市: <input value={cityLocation} onChange={(e) => setCityLocation(e.target.value)} style={{ padding: tokens.spacing.sm, border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.sm }} />
          </label>
          <button onClick={handleSave} style={{ marginRight: tokens.spacing.sm, padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>保存</button>
          <button onClick={() => setEditing(false)} style={{ padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`, background: tokens.color.neutral.surface, border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>取消</button>
        </div>
      ) : (
        <div>
          {profile ? (
            <div>
              <p style={{ color: tokens.color.neutral['text-primary'] }}><strong>名称:</strong> {profile.display_name || '(未填写)'}</p>
              <p style={{ color: tokens.color.neutral['text-primary'] }}><strong>城市:</strong> {profile.city_level_location || '(未填写)'}</p>
              <p style={{ color: tokens.color.neutral['text-primary'] }}><strong>语言:</strong> {profile.languages.join(', ')}</p>
              <p style={{ color: tokens.color.neutral['text-primary'] }}><strong>兴趣:</strong> {profile.traits?.interest_tags?.join(', ') || '(无)'}</p>
              <p style={{ color: tokens.color.neutral['text-primary'] }}><strong>目标:</strong> {profile.traits?.connection_goal_tags?.join(', ') || '(无)'}</p>
              <p style={{ color: tokens.color.neutral['text-primary'] }}><strong>沟通偏好:</strong> {profile.traits?.communication_preferences?.join(', ') || '(无)'}</p>
              <button onClick={() => { setDisplayName(profile.display_name); setCityLocation(profile.city_level_location); setEditing(true); }} style={{
                marginTop: tokens.spacing.md, padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`,
                background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer',
              }}>
                编辑资料
              </button>
            </div>
          ) : (
            <p style={{ color: tokens.color.neutral['text-placeholder'] }}>画像数据暂不可用</p>
          )}
        </div>
      )}
    </div>
  );
}
