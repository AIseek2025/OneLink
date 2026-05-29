import { useEffect, useState } from 'react';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';
import { submitSafetyReport, submitSafetyBlock, fetchAppealStatus } from '../api/client';
import { OlNavBar } from '../components/OlNavBar';

type SafetyView = 'report' | 'block' | 'appeal';

export function SafetyPage() {
  const [view, setView] = useState<SafetyView>('report');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  const [reportTargetId, setReportTargetId] = useState('');
  const [reportCategory, setReportCategory] = useState('');
  const [reportDescription, setReportDescription] = useState('');

  const [blockTargetId, setBlockTargetId] = useState('');
  const [blockReason, setBlockReason] = useState('');

  const [appealReportId, setAppealReportId] = useState('');
  const [appealStatus, setAppealStatus] = useState<{ status: string; reviewed_at: string | null; outcome: string | null } | null>(null);

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    trackEvent({ event_name: 'page.view', page_path: '/safety' }, getAnalyticsContext({ screen: '/safety' }));
  }, []);

  const handleReport = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!reportTargetId.trim() || !reportCategory.trim()) return;
    setLoading(true);
    setError('');
    setSuccess('');
    try {
      await submitSafetyReport(reportTargetId, reportCategory, reportDescription);
      setSuccess('举报已提交，我们将尽快审核');
      setReportTargetId('');
      setReportCategory('');
      setReportDescription('');
      trackEvent({ event_name: 'safety.report.submitted' }, getAnalyticsContext({ screen: '/safety' }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Report failed');
    } finally {
      setLoading(false);
    }
  };

  const handleBlock = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!blockTargetId.trim()) return;
    setLoading(true);
    setError('');
    setSuccess('');
    try {
      await submitSafetyBlock(blockTargetId, blockReason);
      setSuccess('已屏蔽该用户');
      setBlockTargetId('');
      setBlockReason('');
      trackEvent({ event_name: 'safety.block.submitted' }, getAnalyticsContext({ screen: '/safety' }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Block failed');
    } finally {
      setLoading(false);
    }
  };

  const handleAppealLookup = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!appealReportId.trim()) return;
    setLoading(true);
    setError('');
    try {
      const data = await fetchAppealStatus(appealReportId);
      setAppealStatus(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Appeal lookup failed');
    } finally {
      setLoading(false);
    }
  };

  const navItems = [
    { label: '首页', href: '/' },
    { label: '找人', href: '/find' },
    { label: '私信', href: '/dm' },
    { label: '安全', href: '/safety', active: true },
    { label: '设置', href: '/settings' },
  ];

  const handleLogout = () => {
    localStorage.removeItem('onelink_token');
    window.location.href = '/login';
  };

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`,
    background: active ? tokens.color.brand.primary : tokens.color.neutral.surface,
    color: active ? '#FFFFFF' : tokens.color.neutral['text-primary'],
    border: `1px solid ${active ? tokens.color.brand.primary : tokens.color.neutral.border}`,
    borderRadius: tokens.borderRadius.md,
    cursor: 'pointer',
    fontSize: tokens.typography.fontSize.sm,
    fontWeight: active ? tokens.typography.fontWeight.semibold : tokens.typography.fontWeight.normal,
  });

  const inputStyle: React.CSSProperties = {
    width: '100%',
    padding: tokens.spacing.sm,
    fontSize: tokens.typography.fontSize.sm,
    borderRadius: tokens.borderRadius.md,
    border: `1px solid ${tokens.color.neutral.border}`,
  };

  const labelStyle: React.CSSProperties = {
    display: 'block',
    fontSize: tokens.typography.fontSize.sm,
    fontWeight: tokens.typography.fontWeight.medium,
    color: tokens.color.neutral['text-primary'],
    marginBottom: tokens.spacing.xs,
  };

  return (
    <div>
      <OlNavBar items={navItems} onLogout={handleLogout} />
      <div style={{ maxWidth: 640, margin: '0 auto', padding: tokens.spacing.xl }}>
        <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.lg }}>安全中心</h2>

        <div style={{ display: 'flex', gap: tokens.spacing.sm, marginBottom: tokens.spacing.lg }}>
          <button style={tabStyle(view === 'report')} onClick={() => { setView('report'); setError(''); setSuccess(''); }}>举报</button>
          <button style={tabStyle(view === 'block')} onClick={() => { setView('block'); setError(''); setSuccess(''); }}>屏蔽</button>
          <button style={tabStyle(view === 'appeal')} onClick={() => { setView('appeal'); setError(''); setSuccess(''); }}>申诉</button>
        </div>

        {error && (
          <div style={{ padding: tokens.spacing.md, background: tokens.color.semantic['error-bg'], borderRadius: tokens.borderRadius.md, color: tokens.color.semantic.error, marginBottom: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm }}>
            {error}
          </div>
        )}
        {success && (
          <div style={{ padding: tokens.spacing.md, background: tokens.color.semantic['success-bg'] ?? '#D4EDDA', borderRadius: tokens.borderRadius.md, color: tokens.color.semantic.success ?? '#155724', marginBottom: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm }}>
            {success}
          </div>
        )}

        {view === 'report' && (
          <form onSubmit={handleReport} style={{ background: tokens.color.neutral.surface, padding: tokens.spacing.lg, borderRadius: tokens.borderRadius.lg }}>
            <label style={labelStyle}>被举报用户 ID</label>
            <input style={inputStyle} value={reportTargetId} onChange={(e) => setReportTargetId(e.target.value)} placeholder="输入用户 ID" />

            <label style={{ ...labelStyle, marginTop: tokens.spacing.md }}>举报类别</label>
            <select style={inputStyle} value={reportCategory} onChange={(e) => setReportCategory(e.target.value)}>
              <option value="">选择类别</option>
              <option value="harassment">骚扰</option>
              <option value="spam">垃圾信息</option>
              <option value="inappropriate">不当内容</option>
              <option value="fraud">欺诈</option>
              <option value="other">其他</option>
            </select>

            <label style={{ ...labelStyle, marginTop: tokens.spacing.md }}>描述（可选）</label>
            <textarea
              style={{ ...inputStyle, height: 80 }}
              value={reportDescription}
              onChange={(e) => setReportDescription(e.target.value)}
              placeholder="详细描述情况"
            />

            <button type="submit" disabled={loading} style={{ marginTop: tokens.spacing.lg, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: loading ? 'not-allowed' : 'pointer', opacity: loading ? 0.6 : 1 }}>
              {loading ? '提交中...' : '提交举报'}
            </button>
          </form>
        )}

        {view === 'block' && (
          <form onSubmit={handleBlock} style={{ background: tokens.color.neutral.surface, padding: tokens.spacing.lg, borderRadius: tokens.borderRadius.lg }}>
            <label style={labelStyle}>屏蔽用户 ID</label>
            <input style={inputStyle} value={blockTargetId} onChange={(e) => setBlockTargetId(e.target.value)} placeholder="输入用户 ID" />

            <label style={{ ...labelStyle, marginTop: tokens.spacing.md }}>原因（可选）</label>
            <input style={inputStyle} value={blockReason} onChange={(e) => setBlockReason(e.target.value)} placeholder="屏蔽原因" />

            <button type="submit" disabled={loading} style={{ marginTop: tokens.spacing.lg, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: loading ? 'not-allowed' : 'pointer', opacity: loading ? 0.6 : 1 }}>
              {loading ? '处理中...' : '屏蔽用户'}
            </button>
          </form>
        )}

        {view === 'appeal' && (
          <div style={{ background: tokens.color.neutral.surface, padding: tokens.spacing.lg, borderRadius: tokens.borderRadius.lg }}>
            <form onSubmit={handleAppealLookup}>
              <label style={labelStyle}>举报 ID</label>
              <input style={inputStyle} value={appealReportId} onChange={(e) => setAppealReportId(e.target.value)} placeholder="输入举报 ID 查询申诉状态" />
              <button type="submit" disabled={loading} style={{ marginTop: tokens.spacing.md, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: loading ? 'not-allowed' : 'pointer', opacity: loading ? 0.6 : 1 }}>
                {loading ? '查询中...' : '查询状态'}
              </button>
            </form>

            {appealStatus && (
              <div style={{ marginTop: tokens.spacing.lg, padding: tokens.spacing.md, background: tokens.color.neutral['bg-secondary'] ?? '#F5F5F5', borderRadius: tokens.borderRadius.md }}>
                <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-primary'] }}>状态: <strong>{appealStatus.status}</strong></p>
                {appealStatus.reviewed_at && <p style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'] }}>审核时间: {appealStatus.reviewed_at}</p>}
                {appealStatus.outcome && <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-primary'] }}>结果: {appealStatus.outcome}</p>}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
