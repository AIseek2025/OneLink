import { useEffect, useState } from 'react';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';
import { fetchDmThreadList, fetchDmThreadDetail, submitDmFirstMessage } from '../api/client';
import { OlNavBar } from '../components/OlNavBar';

interface DmThread {
  thread_id: string;
  other_user_id: string;
  other_user_name: string;
  last_message_preview: string;
  last_message_at: string;
  state: string;
}

export function DmPage() {
  const [threads, setThreads] = useState<DmThread[]>([]);
  const [selectedThread, setSelectedThread] = useState<DmThread | null>(null);
  const [messages, setMessages] = useState<{ id: string; sender_id: string; content: string; created_at: string }[]>([]);
  const [newMessage, setNewMessage] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [safetyNotice, setSafetyNotice] = useState('');
  const [sendError, setSendError] = useState('');

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    trackEvent({ event_name: 'page.view', page_path: '/dm' }, getAnalyticsContext({ screen: '/dm' }));
    loadThreads();
  }, []);

  const loadThreads = async () => {
    setLoading(true);
    setError('');
    try {
      const data = await fetchDmThreadList();
      setThreads(data.threads ?? data.items ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load threads');
    } finally {
      setLoading(false);
    }
  };

  const selectThread = async (thread: DmThread) => {
    setSelectedThread(thread);
    setSafetyNotice('');
    try {
      const detail = await fetchDmThreadDetail(thread.thread_id);
      setMessages(detail.messages ?? detail.items ?? []);
      if (detail.safety_notice || detail.first_message_state === 'under_review') {
        setSafetyNotice(detail.safety_notice ?? '首条私信正在安全审核中，发送可能受限');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load thread');
    }
  };

  const handleSend = async () => {
    if (!newMessage.trim() || !selectedThread) return;
    setSendError('');
    try {
      await submitDmFirstMessage(selectedThread.thread_id, newMessage.trim());
      setNewMessage('');
      selectThread(selectedThread);
    } catch (e) {
      setSendError(e instanceof Error ? e.message : 'Send failed');
    }
  };

  const navItems = [
    { label: '首页', href: '/' },
    { label: '聊天', href: '/chat' },
    { label: '找人', href: '/find' },
    { label: '私信', href: '/dm', active: true },
    { label: '设置', href: '/settings' },
  ];

  const handleLogout = () => {
    localStorage.removeItem('onelink_token');
    window.location.href = '/login';
  };

  return (
    <div>
      <OlNavBar items={navItems} onLogout={handleLogout} />
      <div style={{ maxWidth: 960, margin: '0 auto', padding: tokens.spacing.xl }}>
        <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.lg }}>私信</h2>

        {loading ? (
          <div style={{ padding: tokens.spacing.xl, textAlign: 'center', color: tokens.color.neutral['text-secondary'] }}>加载中...</div>
        ) : error ? (
          <div style={{ padding: tokens.spacing.lg, background: tokens.color.semantic['error-bg'], borderRadius: tokens.borderRadius.lg, color: tokens.color.semantic.error }}>
            {error}
            <button onClick={loadThreads} style={{ marginLeft: tokens.spacing.md, padding: `${tokens.spacing.xs} ${tokens.spacing.md}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>重试</button>
          </div>
        ) : threads.length === 0 ? (
          <div style={{ padding: tokens.spacing.xl, textAlign: 'center' }}>
            <p style={{ fontSize: tokens.typography.fontSize.lg, color: tokens.color.neutral['text-secondary'] }}>暂无私信</p>
            <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-placeholder'] }}>通过推荐列表发起私信</p>
            <a href="/recommendations" style={{ display: 'inline-block', marginTop: tokens.spacing.lg, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', borderRadius: tokens.borderRadius.md, textDecoration: 'none' }}>查看推荐</a>
          </div>
        ) : (
          <div style={{ display: 'flex', gap: tokens.spacing.lg }}>
            <div style={{ flex: '1 1 300px' }}>
              {threads.map((t) => (
                <div
                  key={t.thread_id}
                  onClick={() => selectThread(t)}
                  style={{
                    padding: tokens.spacing.md,
                    background: selectedThread?.thread_id === t.thread_id ? tokens.color.brand['primary-light'] : tokens.color.neutral.surface,
                    borderRadius: tokens.borderRadius.md,
                    border: `1px solid ${tokens.color.neutral.border}`,
                    marginBottom: tokens.spacing.sm,
                    cursor: 'pointer',
                  }}
                >
                  <div style={{ fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], fontSize: tokens.typography.fontSize.sm }}>{t.other_user_name || t.other_user_id}</div>
                  <div style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'], marginTop: tokens.spacing.xs }}>{t.last_message_preview || '暂无消息'}</div>
                </div>
              ))}
            </div>

            {selectedThread && (
              <div style={{ flex: '2 1 500px', background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg, padding: tokens.spacing.lg }}>
                <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.md }}>
                  {selectedThread.other_user_name || selectedThread.other_user_id}
                </h3>

                {safetyNotice && (
                  <div style={{ padding: tokens.spacing.md, background: tokens.color.semantic['warning-bg'] ?? '#FFF3CD', borderRadius: tokens.borderRadius.md, marginBottom: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm, color: tokens.color.semantic.warning }}>
                    {safetyNotice}
                  </div>
                )}

                <div style={{ maxHeight: 400, overflowY: 'auto', marginBottom: tokens.spacing.md }}>
                  {messages.length === 0 ? (
                    <p style={{ color: tokens.color.neutral['text-placeholder'], fontSize: tokens.typography.fontSize.sm, textAlign: 'center' }}>对话为空</p>
                  ) : (
                    messages.map((m) => (
                      <div key={m.id} style={{ marginBottom: tokens.spacing.sm }}>
                        <span style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'] }}>{m.sender_id}</span>
                        <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-primary'] }}>{m.content}</p>
                      </div>
                    ))
                  )}
                </div>

                <div style={{ display: 'flex', gap: tokens.spacing.sm }}>
                  <input
                    type="text"
                    value={newMessage}
                    onChange={(e) => setNewMessage(e.target.value)}
                    placeholder="输入消息..."
                    style={{ flex: 1, padding: tokens.spacing.sm, fontSize: tokens.typography.fontSize.sm, borderRadius: tokens.borderRadius.md, border: `1px solid ${tokens.color.neutral.border}` }}
                    onKeyDown={(e) => { if (e.key === 'Enter') handleSend(); }}
                  />
                  <button onClick={handleSend} style={{ padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>
                    发送
                  </button>
                </div>
                {sendError && <p style={{ color: tokens.color.semantic.error, fontSize: tokens.typography.fontSize.xs, marginTop: tokens.spacing.xs }}>{sendError}</p>}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
