import { useEffect, useState } from 'react';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';
import { OlNavBar } from '../components/OlNavBar';
import { fetchAdminReportQueue, fetchAdminAppealQueue, submitAdminReportAction, fetchAdminMetrics } from '../api/client';

interface Metrics {
  total_users: number;
  active_users_7d: number;
  total_matches: number;
  total_dms: number;
  pending_reports: number;
  pending_appeals: number;
}

export function AdminPage() {
  const [reports, setReports] = useState<{ id: string; target_user_id: string; category: string; status: string; created_at: string }[]>([]);
  const [appeals, setAppeals] = useState<{ id: string; user_id: string; status: string; created_at: string }[]>([]);
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [activeTab, setActiveTab] = useState<'metrics' | 'reports' | 'appeals'>('metrics');

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    trackEvent({ event_name: 'page.view', page_path: '/admin' }, getAnalyticsContext({ screen: '/admin' }));
    loadData();
  }, []);

  const loadData = async () => {
    setLoading(true);
    setError('');
    try {
      const [reportData, appealData, metricsData] = await Promise.all([
        fetchAdminReportQueue(),
        fetchAdminAppealQueue(),
        fetchAdminMetrics().catch(() => null),
      ]);
      setReports(reportData.items ?? reportData.reports ?? []);
      setAppeals(appealData.items ?? appealData.appeals ?? []);
      if (metricsData) setMetrics(metricsData);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load admin data');
    } finally {
      setLoading(false);
    }
  };

  const handleAction = async (reportId: string, actionType: string) => {
    try {
      await submitAdminReportAction(reportId, actionType);
      loadData();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Action failed');
    }
  };

  const navItems = [
    { label: '首页', href: '/' },
    { label: '聊天', href: '/chat' },
    { label: '画像', href: '/profile' },
    { label: '找人', href: '/find' },
    { label: '管理', href: '/admin', active: true },
  ];

  const handleLogout = () => {
    localStorage.removeItem('onelink_token');
    window.location.href = '/login';
  };

  const cardStyle: React.CSSProperties = {
    padding: tokens.spacing.lg,
    background: tokens.color.neutral.surface,
    borderRadius: tokens.borderRadius.lg,
    border: `1px solid ${tokens.color.neutral.border}`,
    marginBottom: tokens.spacing.md,
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

  const metricCardStyle: React.CSSProperties = {
    padding: tokens.spacing.lg,
    background: tokens.color.neutral.surface,
    borderRadius: tokens.borderRadius.lg,
    border: `1px solid ${tokens.color.neutral.border}`,
    textAlign: 'center',
  };

  return (
    <div>
      <OlNavBar items={navItems} onLogout={handleLogout} />
      <div style={{ maxWidth: 960, margin: '0 auto', padding: tokens.spacing.xl }}>
        <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.lg }}>管理后台</h2>

        <div style={{ display: 'flex', gap: tokens.spacing.sm, marginBottom: tokens.spacing.lg }}>
          <button style={tabStyle(activeTab === 'metrics')} onClick={() => setActiveTab('metrics')}>指标看板</button>
          <button style={tabStyle(activeTab === 'reports')} onClick={() => setActiveTab('reports')}>举报队列 ({reports.length})</button>
          <button style={tabStyle(activeTab === 'appeals')} onClick={() => setActiveTab('appeals')}>申诉队列 ({appeals.length})</button>
        </div>

        {loading ? (
          <div style={{ padding: tokens.spacing.xl, textAlign: 'center', color: tokens.color.neutral['text-secondary'] }}>加载中...</div>
        ) : error ? (
          <div style={{ padding: tokens.spacing.lg, background: tokens.color.semantic['error-bg'], borderRadius: tokens.borderRadius.lg, color: tokens.color.semantic.error }}>
            {error}
            <button onClick={loadData} style={{ marginLeft: tokens.spacing.md, padding: `${tokens.spacing.xs} ${tokens.spacing.md}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>重试</button>
          </div>
        ) : activeTab === 'metrics' ? (
          metrics ? (
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: tokens.spacing.md }}>
              {[
                { label: '总用户数', value: metrics.total_users },
                { label: '7日活跃', value: metrics.active_users_7d },
                { label: '总匹配数', value: metrics.total_matches },
                { label: '总私信数', value: metrics.total_dms },
                { label: '待处理举报', value: metrics.pending_reports },
                { label: '待处理申诉', value: metrics.pending_appeals },
              ].map(({ label, value }) => (
                <div key={label} style={metricCardStyle}>
                  <div style={{ fontSize: tokens.typography.fontSize.xl, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.brand.primary }}>{value ?? 0}</div>
                  <div style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'], marginTop: tokens.spacing.xs }}>{label}</div>
                </div>
              ))}
            </div>
          ) : (
            <div style={{ padding: tokens.spacing.xl, textAlign: 'center' }}>
              <p style={{ color: tokens.color.neutral['text-secondary'] }}>暂无指标数据</p>
            </div>
          )
        ) : activeTab === 'reports' ? (
          reports.length === 0 ? (
            <p style={{ color: tokens.color.neutral['text-secondary'], textAlign: 'center', padding: tokens.spacing.xl }}>暂无待处理举报</p>
          ) : (
            reports.map((r) => (
              <div key={r.id} style={cardStyle}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: tokens.spacing.sm }}>
                  <span style={{ fontSize: tokens.typography.fontSize.sm, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'] }}>举报 {r.id}</span>
                  <span style={{ fontSize: tokens.typography.fontSize.xs, color: r.status === 'pending' ? tokens.color.semantic.warning : tokens.color.neutral['text-secondary'] }}>{r.status}</span>
                </div>
                <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>用户: {r.target_user_id} | 类别: {r.category}</p>
                {r.status === 'pending' && (
                  <div style={{ marginTop: tokens.spacing.sm, display: 'flex', gap: tokens.spacing.sm }}>
                    <button onClick={() => handleAction(r.id, 'ban')} style={{ padding: `${tokens.spacing.xs} ${tokens.spacing.md}`, background: tokens.color.semantic.error, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer', fontSize: tokens.typography.fontSize.xs }}>封禁</button>
                    <button onClick={() => handleAction(r.id, 'dismiss')} style={{ padding: `${tokens.spacing.xs} ${tokens.spacing.md}`, background: tokens.color.neutral.surface, color: tokens.color.neutral['text-primary'], border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.md, cursor: 'pointer', fontSize: tokens.typography.fontSize.xs }}>驳回</button>
                  </div>
                )}
              </div>
            ))
          )
        ) : (
          appeals.length === 0 ? (
            <p style={{ color: tokens.color.neutral['text-secondary'], textAlign: 'center', padding: tokens.spacing.xl }}>暂无待处理申诉</p>
          ) : (
            appeals.map((a) => (
              <div key={a.id} style={cardStyle}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontSize: tokens.typography.fontSize.sm, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'] }}>申诉 {a.id}</span>
                  <span style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'] }}>{a.status}</span>
                </div>
                <p style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'], marginTop: tokens.spacing.xs }}>用户: {a.user_id}</p>
              </div>
            ))
          )
        )}
      </div>
    </div>
  );
}
