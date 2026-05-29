import { useEffect, useState } from 'react';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';
import { OlNavBar } from '../components/OlNavBar';
import { fetchComplianceData, requestComplianceExportRaw, requestComplianceCorrectionRaw, requestComplianceDeletion } from '../api/client';

type ComplianceView = 'view' | 'export' | 'correct' | 'delete';

interface PersonalData {
  user_id: string;
  email: string;
  display_name: string;
  primary_region: string;
  primary_language: string;
  created_at: string;
  profile: Record<string, unknown> | null;
  recommendations_count: number;
  dm_threads_count: number;
}

export function CompliancePage() {
  const [view, setView] = useState<ComplianceView>('view');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  const [personalData, setPersonalData] = useState<PersonalData | null>(null);

  const [correctField, setCorrectField] = useState('');
  const [correctValue, setCorrectValue] = useState('');

  const [deleteConfirm, setDeleteConfirm] = useState('');
  const [deleteStep, setDeleteStep] = useState(1);

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    trackEvent({ event_name: 'page.view', page_path: '/compliance' }, getAnalyticsContext({ screen: '/compliance' }));
    loadPersonalData();
  }, []);

  const loadPersonalData = async () => {
    setLoading(true);
    setError('');
    try {
      const data = await fetchComplianceData();
      setPersonalData(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load personal data');
    } finally {
      setLoading(false);
    }
  };

  const handleExport = async () => {
    setLoading(true);
    setError('');
    try {
      const res = await requestComplianceExportRaw();
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'onelink_personal_data.json';
      a.click();
      URL.revokeObjectURL(url);
      setSuccess('数据导出成功');
      trackEvent({ event_name: 'compliance.export.completed' }, getAnalyticsContext({ screen: '/compliance' }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Export failed');
    } finally {
      setLoading(false);
    }
  };

  const handleCorrect = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!correctField.trim() || !correctValue.trim()) return;
    setLoading(true);
    setError('');
    setSuccess('');
    try {
      await requestComplianceCorrectionRaw(correctField, correctValue);
      setSuccess('更正请求已提交');
      setCorrectField('');
      setCorrectValue('');
      trackEvent({ event_name: 'compliance.correction.submitted' }, getAnalyticsContext({ screen: '/compliance' }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Correction failed');
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async () => {
    if (deleteStep === 1) {
      setDeleteStep(2);
      return;
    }
    if (deleteConfirm !== 'DELETE') return;
    setLoading(true);
    setError('');
    try {
      await requestComplianceDeletion(deleteConfirm);
      setSuccess('账户删除请求已提交，数据将在审核后删除');
      setDeleteStep(1);
      setDeleteConfirm('');
      trackEvent({ event_name: 'compliance.deletion.submitted' }, getAnalyticsContext({ screen: '/compliance' }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Deletion failed');
    } finally {
      setLoading(false);
    }
  };

  const navItems = [
    { label: '首页', href: '/' },
    { label: '安全', href: '/safety' },
    { label: '合规', href: '/compliance', active: true },
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
        <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.lg }}>数据合规</h2>
        <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.lg }}>根据 GDPR/个人信息保护法，您可以查看、导出、更正和删除您的个人数据</p>

        <div style={{ display: 'flex', gap: tokens.spacing.sm, marginBottom: tokens.spacing.lg }}>
          <button style={tabStyle(view === 'view')} onClick={() => { setView('view'); setError(''); setSuccess(''); }}>查看数据</button>
          <button style={tabStyle(view === 'export')} onClick={() => { setView('export'); setError(''); setSuccess(''); }}>导出数据</button>
          <button style={tabStyle(view === 'correct')} onClick={() => { setView('correct'); setError(''); setSuccess(''); }}>更正数据</button>
          <button style={tabStyle(view === 'delete')} onClick={() => { setView('delete'); setError(''); setSuccess(''); }}>删除数据</button>
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

        {view === 'view' && (
          <div style={{ background: tokens.color.neutral.surface, padding: tokens.spacing.lg, borderRadius: tokens.borderRadius.lg }}>
            {loading ? (
              <p style={{ color: tokens.color.neutral['text-secondary'], textAlign: 'center' }}>加载中...</p>
            ) : personalData ? (
              <div>
                {[
                  ['用户 ID', personalData.user_id],
                  ['邮箱', personalData.email],
                  ['昵称', personalData.display_name],
                  ['地区', personalData.primary_region],
                  ['语言', personalData.primary_language],
                  ['注册时间', personalData.created_at],
                  ['推荐次数', String(personalData.recommendations_count)],
                  ['私信线程数', String(personalData.dm_threads_count)],
                ].map(([label, value]) => (
                  <div key={label} style={{ display: 'flex', justifyContent: 'space-between', padding: `${tokens.spacing.sm} 0`, borderBottom: `1px solid ${tokens.color.neutral.border}` }}>
                    <span style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>{label}</span>
                    <span style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-primary'], fontWeight: tokens.typography.fontWeight.medium }}>{value ?? '-'}</span>
                  </div>
                ))}
              </div>
            ) : (
              <p style={{ color: tokens.color.neutral['text-secondary'] }}>暂无数据</p>
            )}
          </div>
        )}

        {view === 'export' && (
          <div style={{ background: tokens.color.neutral.surface, padding: tokens.spacing.lg, borderRadius: tokens.borderRadius.lg }}>
            <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.md }}>
              导出您在 OneLink 的全部个人数据，格式为 JSON 文件
            </p>
            <button onClick={handleExport} disabled={loading} style={{ padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: loading ? 'not-allowed' : 'pointer', opacity: loading ? 0.6 : 1 }}>
              {loading ? '导出中...' : '导出我的数据'}
            </button>
          </div>
        )}

        {view === 'correct' && (
          <form onSubmit={handleCorrect} style={{ background: tokens.color.neutral.surface, padding: tokens.spacing.lg, borderRadius: tokens.borderRadius.lg }}>
            <label style={labelStyle}>需要更正的字段</label>
            <select style={inputStyle} value={correctField} onChange={(e) => setCorrectField(e.target.value)}>
              <option value="">选择字段</option>
              <option value="display_name">昵称</option>
              <option value="email">邮箱</option>
              <option value="primary_region">地区</option>
              <option value="primary_language">语言</option>
              <option value="profile">个人画像</option>
            </select>

            <label style={{ ...labelStyle, marginTop: tokens.spacing.md }}>更正后的值</label>
            <textarea
              style={{ ...inputStyle, height: 80 }}
              value={correctValue}
              onChange={(e) => setCorrectValue(e.target.value)}
              placeholder="输入更正后的内容"
            />

            <button type="submit" disabled={loading} style={{ marginTop: tokens.spacing.lg, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: loading ? 'not-allowed' : 'pointer', opacity: loading ? 0.6 : 1 }}>
              {loading ? '提交中...' : '提交更正请求'}
            </button>
          </form>
        )}

        {view === 'delete' && (
          <div style={{ background: tokens.color.neutral.surface, padding: tokens.spacing.lg, borderRadius: tokens.borderRadius.lg }}>
            {deleteStep === 1 ? (
              <div>
                <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.semantic.error, fontWeight: tokens.typography.fontWeight.semibold, marginBottom: tokens.spacing.md }}>
                  警告：删除账户将永久移除您的所有数据，此操作不可逆
                </p>
                <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.md }}>
                  包括：个人画像、找人记录、推荐历史、私信记录、安全举报记录等
                </p>
                <button onClick={handleDelete} style={{ padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.semantic.error, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>
                  我要删除账户
                </button>
              </div>
            ) : (
              <div>
                <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.semantic.error, fontWeight: tokens.typography.fontWeight.semibold, marginBottom: tokens.spacing.md }}>
                  请输入 DELETE 确认删除
                </p>
                <input style={inputStyle} value={deleteConfirm} onChange={(e) => setDeleteConfirm(e.target.value)} placeholder='输入 "DELETE" 确认' />
                <div style={{ display: 'flex', gap: tokens.spacing.md, marginTop: tokens.spacing.md }}>
                  <button onClick={handleDelete} disabled={loading || deleteConfirm !== 'DELETE'} style={{ padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.semantic.error, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: (loading || deleteConfirm !== 'DELETE') ? 'not-allowed' : 'pointer', opacity: (loading || deleteConfirm !== 'DELETE') ? 0.6 : 1 }}>
                    {loading ? '处理中...' : '确认永久删除'}
                  </button>
                  <button onClick={() => { setDeleteStep(1); setDeleteConfirm(''); }} style={{ padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.neutral.surface, color: tokens.color.neutral['text-primary'], border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>
                    取消
                  </button>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}