import { useEffect, useState } from 'react';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';
import { OlNavBar } from '../components/OlNavBar';
import { OlButton } from '../components/OlButton';
import { fetchUserMe, patchUserSettings, patchLocaleSettings } from '../api/client';

export function SettingsPage() {
  const [locale, setLocale] = useState('zh-CN');
  const [region, setRegion] = useState('CN');
  const [timezone, setTimezone] = useState('Asia/Shanghai');
  const [notificationLanguage, setNotificationLanguage] = useState('zh-CN');
  const [allowSearch, setAllowSearch] = useState(true);
  const [allowRecommend, setAllowRecommend] = useState(true);
  const [saving, setSaving] = useState(false);
  const [success, setSuccess] = useState('');
  const [error, setError] = useState('');

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    trackEvent({ event_name: 'page.view', page_path: '/settings' }, getAnalyticsContext({ screen: '/settings' }));
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const data = await fetchUserMe();
      if (data.locale) setLocale(data.locale);
      if (data.primary_region) setRegion(data.primary_region);
      if (data.timezone) setTimezone(data.timezone);
      if (data.notification_language) setNotificationLanguage(data.notification_language);
      if (data.allow_search !== undefined) setAllowSearch(data.allow_search);
      if (data.allow_recommend !== undefined) setAllowRecommend(data.allow_recommend);
    } catch {
      // use defaults
    }
  };

  const handleSaveLocale = async (fields: Record<string, string>) => {
    setSaving(true);
    setError('');
    setSuccess('');
    try {
      await patchLocaleSettings(fields);
      setSuccess('设置已保存');
      trackEvent({ event_name: 'settings.saved' }, getAnalyticsContext({ screen: '/settings' }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  const handleSavePrivacy = async (fields: Record<string, unknown>) => {
    setSaving(true);
    setError('');
    setSuccess('');
    try {
      await patchUserSettings(fields);
      setSuccess('设置已保存');
      trackEvent({ event_name: 'settings.saved' }, getAnalyticsContext({ screen: '/settings' }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  const navItems = [
    { label: '首页', href: '/' },
    { label: '聊天', href: '/chat' },
    { label: '画像', href: '/profile' },
    { label: '找人', href: '/find' },
    { label: '设置', href: '/settings', active: true },
  ];

  const handleLogout = () => {
    localStorage.removeItem('onelink_token');
    window.location.href = '/login';
  };

  const sectionStyle: React.CSSProperties = {
    padding: tokens.spacing.lg,
    background: tokens.color.neutral.surface,
    borderRadius: tokens.borderRadius.lg,
    marginBottom: tokens.spacing.md,
  };

  const labelStyle: React.CSSProperties = {
    display: 'block',
    fontSize: tokens.typography.fontSize.sm,
    fontWeight: tokens.typography.fontWeight.medium,
    color: tokens.color.neutral['text-primary'],
    marginBottom: tokens.spacing.xs,
  };

  const selectStyle: React.CSSProperties = {
    width: '100%',
    padding: tokens.spacing.sm,
    fontSize: tokens.typography.fontSize.sm,
    borderRadius: tokens.borderRadius.md,
    border: `1px solid ${tokens.color.neutral.border}`,
    background: tokens.color.neutral.bg,
    color: tokens.color.neutral['text-primary'],
  };

  const toggleStyle = (on: boolean): React.CSSProperties => ({
    width: 44,
    height: 24,
    borderRadius: 12,
    background: on ? tokens.color.brand.primary : tokens.color.neutral.border,
    position: 'relative',
    cursor: 'pointer',
    transition: 'background 0.2s',
    flexShrink: 0,
  });

  const toggleKnobStyle = (on: boolean): React.CSSProperties => ({
    width: 20,
    height: 20,
    borderRadius: 10,
    background: '#FFFFFF',
    position: 'absolute',
    top: 2,
    left: on ? 22 : 2,
    transition: 'left 0.2s',
  });

  return (
    <div>
      <OlNavBar items={navItems} onLogout={handleLogout} />
      <div style={{ maxWidth: 640, margin: '0 auto', padding: tokens.spacing.xl }}>
        <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.lg }}>设置</h2>

        {error && (
          <div style={{ padding: tokens.spacing.md, background: tokens.color.semantic['error-bg'], borderRadius: tokens.borderRadius.md, color: tokens.color.semantic.error, marginBottom: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm }}>{error}</div>
        )}
        {success && (
          <div style={{ padding: tokens.spacing.md, background: tokens.color.semantic['success-bg'] ?? '#D4EDDA', borderRadius: tokens.borderRadius.md, color: tokens.color.semantic.success ?? '#155724', marginBottom: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm }}>{success}</div>
        )}

        <div style={sectionStyle}>
          <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], margin: `0 0 ${tokens.spacing.md} 0` }}>区域与语言</h3>

          <div style={{ marginBottom: tokens.spacing.md }}>
            <label style={labelStyle}>界面语言 (Locale)</label>
            <select style={selectStyle} value={locale} onChange={(e) => setLocale(e.target.value)} onBlur={() => handleSaveLocale({ locale })}>
              <option value="zh-CN">简体中文</option>
              <option value="zh-TW">繁體中文</option>
              <option value="en-US">English (US)</option>
              <option value="en-GB">English (UK)</option>
              <option value="ja-JP">日本語</option>
              <option value="ko-KR">한국어</option>
            </select>
          </div>

          <div style={{ marginBottom: tokens.spacing.md }}>
            <label style={labelStyle}>地区 (Region)</label>
            <select style={selectStyle} value={region} onChange={(e) => setRegion(e.target.value)} onBlur={() => handleSaveLocale({ region })}>
              <option value="CN">中国大陆</option>
              <option value="HK">香港</option>
              <option value="TW">台湾</option>
              <option value="SG">新加坡</option>
              <option value="JP">日本</option>
              <option value="KR">韩国</option>
              <option value="US">美国</option>
              <option value="GB">英国</option>
            </select>
          </div>

          <div style={{ marginBottom: tokens.spacing.md }}>
            <label style={labelStyle}>时区 (Timezone)</label>
            <select style={selectStyle} value={timezone} onChange={(e) => setTimezone(e.target.value)} onBlur={() => handleSaveLocale({ timezone })}>
              <option value="Asia/Shanghai">Asia/Shanghai (UTC+8)</option>
              <option value="Asia/Hong_Kong">Asia/Hong_Kong (UTC+8)</option>
              <option value="Asia/Taipei">Asia/Taipei (UTC+8)</option>
              <option value="Asia/Tokyo">Asia/Tokyo (UTC+9)</option>
              <option value="Asia/Seoul">Asia/Seoul (UTC+9)</option>
              <option value="Asia/Singapore">Asia/Singapore (UTC+8)</option>
              <option value="America/New_York">America/New_York (UTC-5)</option>
              <option value="America/Los_Angeles">America/Los_Angeles (UTC-8)</option>
              <option value="Europe/London">Europe/London (UTC+0)</option>
            </select>
          </div>

          <div>
            <label style={labelStyle}>通知语言 (Notification Language)</label>
            <select style={selectStyle} value={notificationLanguage} onChange={(e) => setNotificationLanguage(e.target.value)} onBlur={() => handleSaveLocale({ notification_language: notificationLanguage })}>
              <option value="zh-CN">简体中文</option>
              <option value="zh-TW">繁體中文</option>
              <option value="en-US">English</option>
              <option value="ja-JP">日本語</option>
              <option value="ko-KR">한국어</option>
            </select>
          </div>
        </div>

        <div style={sectionStyle}>
          <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], margin: `0 0 ${tokens.spacing.md} 0` }}>隐私</h3>

          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: tokens.spacing.md }}>
            <div>
              <div style={labelStyle}>允许被搜索</div>
              <div style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'] }}>其他用户可以通过条件搜索到您</div>
            </div>
            <div style={toggleStyle(allowSearch)} onClick={() => { const v = !allowSearch; setAllowSearch(v); handleSavePrivacy({ allow_search: v }); }}>
              <div style={toggleKnobStyle(allowSearch)} />
            </div>
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <div style={labelStyle}>允许被推荐</div>
              <div style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'] }}>系统会将您推荐给其他用户</div>
            </div>
            <div style={toggleStyle(allowRecommend)} onClick={() => { const v = !allowRecommend; setAllowRecommend(v); handleSavePrivacy({ allow_recommend: v }); }}>
              <div style={toggleKnobStyle(allowRecommend)} />
            </div>
          </div>
        </div>

        <div style={sectionStyle}>
          <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], margin: `0 0 ${tokens.spacing.md} 0` }}>数据与安全</h3>
          <a href="/compliance" style={{ display: 'block', fontSize: tokens.typography.fontSize.sm, color: tokens.color.brand.primary, textDecoration: 'none', marginBottom: tokens.spacing.sm }}>数据合规中心</a>
          <a href="/safety" style={{ display: 'block', fontSize: tokens.typography.fontSize.sm, color: tokens.color.brand.primary, textDecoration: 'none' }}>安全中心</a>
        </div>

        <div style={sectionStyle}>
          <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], margin: `0 0 ${tokens.spacing.md} 0` }}>帮助与支持</h3>
          <a href="mailto:support@onelink.app" style={{ display: 'block', fontSize: tokens.typography.fontSize.sm, color: tokens.color.brand.primary, textDecoration: 'none', marginBottom: tokens.spacing.sm }}>联系客服</a>
          <a href="/faq" style={{ display: 'block', fontSize: tokens.typography.fontSize.sm, color: tokens.color.brand.primary, textDecoration: 'none', marginBottom: tokens.spacing.sm }}>常见问题</a>
          <a href="/terms" style={{ display: 'block', fontSize: tokens.typography.fontSize.sm, color: tokens.color.brand.primary, textDecoration: 'none' }}>使用条款与隐私政策</a>
        </div>

        <div style={sectionStyle}>
          <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], margin: `0 0 ${tokens.spacing.md} 0` }}>关于</h3>
          <div>
            <div style={labelStyle}>版本</div>
            <div style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>0.1.0</div>
          </div>
        </div>

        <OlButton variant="danger" fullWidth onClick={handleLogout} disabled={saving}>
          退出登录
        </OlButton>
      </div>
    </div>
  );
}