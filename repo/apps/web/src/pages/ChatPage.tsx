import { useEffect, useState, useCallback } from 'react';
import { fetchChatInit, sendMessage, fetchDmThreadList, fetchDmThreadDetail, submitDmFirstMessage } from '../api/client';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';

interface ChatInitData {
  user: { user_id: string };
  conversation: { conversation_id: string; status: string };
  pending_questions: Array<{ delivery_id: string; question_text: string }>;
}

interface Message {
  id: string;
  text: string;
  sender: 'user' | 'ai';
}

interface DmThread {
  thread_id: string;
  other_user_id: string;
  other_user_name: string;
  last_message_preview: string;
  last_message_at: string;
  state: string;
}

type ChatView = 'lumi' | 'dm';

export function ChatPage() {
  const [data, setData] = useState<ChatInitData | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [error, setError] = useState('');
  const [chatView, setChatView] = useState<ChatView>('lumi');

  const [dmThreads, setDmThreads] = useState<DmThread[]>([]);
  const [dmMessages, setDmMessages] = useState<{ id: string; sender_id: string; content: string; created_at: string }[]>([]);
  const [selectedDmThread, setSelectedDmThread] = useState<DmThread | null>(null);
  const [dmInput, setDmInput] = useState('');
  const [dmLoading, setDmLoading] = useState(false);
  const [dmError, setDmError] = useState('');
  const [safetyNotice, setSafetyNotice] = useState('');
const [dmSendError, setDmSendError] = useState('');

const [showQuestions, setShowQuestions] = useState(false);

  const loadChatInit = useCallback(async () => {
    try {
      const d = await fetchChatInit();
      setData(d);
      trackEvent({ event_name: 'page.view', page_path: '/chat' }, getAnalyticsContext({ screen: '/chat', user_id: d.user?.user_id ?? null }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to init chat');
    }
  }, []);

  const loadDmThreads = useCallback(async () => {
    setDmLoading(true);
    setDmError('');
    try {
      const res = await fetchDmThreadList();
      setDmThreads(res.threads ?? res.items ?? []);
    } catch (e) {
      setDmError(e instanceof Error ? e.message : 'Failed to load DM threads');
    } finally {
      setDmLoading(false);
    }
  }, []);

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    loadChatInit();
    loadDmThreads();
  }, [loadChatInit, loadDmThreads]);

  const handleSend = async () => {
    if (!input.trim() || !data) return;
    const userMsg: Message = { id: Date.now().toString(), text: input, sender: 'user' };
    setMessages((prev) => [...prev, userMsg]);
    const sentAt = Date.now();
    setInput('');
    setError('');
    try {
      const res = await sendMessage(data.conversation.conversation_id, input);
      const aiMsg: Message = { id: res.ai_message_id, text: res.ai_content_text, sender: 'ai' };
      setMessages((prev) => [...prev, aiMsg]);
      const ctx = getAnalyticsContext({ screen: '/chat', user_id: data.user?.user_id ?? null });
      trackEvent({ event_name: 'chat.message.sent', user_id: data.user?.user_id ?? '', conversation_id: data.conversation.conversation_id, content_type: 'text' }, ctx);
      trackEvent({ event_name: 'chat.message.received', user_id: data.user?.user_id ?? '', conversation_id: data.conversation.conversation_id, response_latency_ms: Date.now() - sentAt }, ctx);
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Send failed';
      setError(msg);
      trackEvent({ event_name: 'error.occurred', error_type: 'network', error_message: msg }, getAnalyticsContext({ screen: '/chat' }));
    }
  };

  const handleChatRetry = () => {
    setError('');
    if (!data) { loadChatInit(); return; }
    handleSend();
  };

  const selectDmThread = async (thread: DmThread) => {
    setSelectedDmThread(thread);
    setSafetyNotice('');
    setDmError('');
    try {
      const detail = await fetchDmThreadDetail(thread.thread_id);
      setDmMessages(detail.messages ?? detail.items ?? []);
      if (detail.safety_notice || detail.first_message_state === 'under_review') {
        setSafetyNotice(detail.safety_notice ?? '首条私信正在安全审核中，发送可能受限');
      }
    } catch (e) {
      setDmError(e instanceof Error ? e.message : 'Failed to load thread');
    }
  };

  const handleDmSend = async () => {
    if (!dmInput.trim() || !selectedDmThread) return;
    setDmSendError('');
    try {
      await submitDmFirstMessage(selectedDmThread.thread_id, dmInput.trim());
      setDmInput('');
      selectDmThread(selectedDmThread);
    } catch (e) {
      setDmSendError(e instanceof Error ? e.message : 'Send failed');
    }
  };

  const handleDmRetry = () => {
    setDmSendError('');
    handleDmSend();
  };

  if (error && !data) return (
    <div style={{ padding: tokens.spacing.xl, textAlign: 'center' }}>
      <p style={{ color: tokens.color.semantic.error, marginBottom: tokens.spacing.md }}>{error}</p>
      <button onClick={loadChatInit} style={{ padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>重试</button>
    </div>
  );
  if (!data) return <div style={{ padding: tokens.spacing.xl, color: tokens.color.neutral['text-secondary'] }}>Loading...</div>;

  const hasPendingQuestions = data.pending_questions && data.pending_questions.length > 0;

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`,
    background: active ? tokens.color.brand.primary : 'transparent',
    color: active ? '#FFFFFF' : tokens.color.neutral['text-secondary'],
    border: 'none',
    borderRadius: tokens.borderRadius.md,
    cursor: 'pointer',
    fontSize: tokens.typography.fontSize.sm,
    fontWeight: active ? tokens.typography.fontWeight.semibold : tokens.typography.fontWeight.normal,
  });

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, borderBottom: `1px solid ${tokens.color.neutral.border}`, background: tokens.color.neutral.bg }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.md }}>
          <h2 style={{ color: tokens.color.brand.primary, fontSize: tokens.typography.fontSize.xl, margin: 0 }}>Lumi Chat</h2>
          <div style={{ display: 'flex', gap: tokens.spacing.xs }}>
            <button style={tabStyle(chatView === 'lumi')} onClick={() => setChatView('lumi')}>AI 对话</button>
            <button style={tabStyle(chatView === 'dm')} onClick={() => setChatView('dm')}>私信 ({dmThreads.length})</button>
          </div>
        </div>
        {chatView === 'lumi' && hasPendingQuestions && (
          <button
            onClick={() => setShowQuestions(!showQuestions)}
            style={{ padding: `${tokens.spacing.xs} ${tokens.spacing.md}`, background: tokens.color.semantic['info-bg'], color: tokens.color.brand.primary, border: `1px solid ${tokens.color.brand.primary}`, borderRadius: tokens.borderRadius.md, cursor: 'pointer', fontSize: tokens.typography.fontSize.xs, fontWeight: tokens.typography.fontWeight.medium }}
          >
            {showQuestions ? '隐藏问题' : `${data.pending_questions.length} 个待答问题`}
          </button>
        )}
      </div>

      {chatView === 'lumi' ? (
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
          <div style={{ flex: 1, maxWidth: 720, margin: '0 auto', padding: tokens.spacing.lg, display: 'flex', flexDirection: 'column' }}>
            {showQuestions && hasPendingQuestions && (
              <div style={{ marginBottom: tokens.spacing.md, padding: tokens.spacing.md, background: tokens.color.semantic['info-bg'], borderRadius: tokens.borderRadius.lg, border: `1px solid ${tokens.color.brand['primary-light']}` }}>
                <p style={{ margin: `0 0 ${tokens.spacing.sm} 0`, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], fontSize: tokens.typography.fontSize.sm }}>Lumi 想了解你更多</p>
                {data.pending_questions.map((q, i) => (
                  <div key={q.delivery_id} style={{ padding: tokens.spacing.sm, marginBottom: i < data.pending_questions.length - 1 ? tokens.spacing.xs : 0, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.md, fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-primary'] }}>
                    {q.question_text}
                  </div>
                ))}
                <a href="/questionnaire" style={{ display: 'inline-block', marginTop: tokens.spacing.sm, fontSize: tokens.typography.fontSize.xs, color: tokens.color.brand.primary, textDecoration: 'none', fontWeight: tokens.typography.fontWeight.medium }}>去回答 →</a>
              </div>
            )}
            <div style={{ flex: 1, overflowY: 'auto', border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.lg, padding: tokens.spacing.md, marginBottom: tokens.spacing.md }}>
              {messages.length === 0 && <p style={{ color: tokens.color.neutral['text-placeholder'] }}>和 Lumi 开始对话吧</p>}
              {messages.map((m) => (
                <div key={m.id} style={{ marginBottom: tokens.spacing.sm, textAlign: m.sender === 'user' ? 'right' : 'left' }}>
                  <span style={{ display: 'inline-block', padding: `${tokens.spacing.md} ${tokens.spacing.lg}`, borderRadius: tokens.borderRadius.lg, background: m.sender === 'user' ? tokens.color.chat['user-bubble'] : tokens.color.chat['ai-bubble'], color: m.sender === 'user' ? tokens.color.chat['user-bubble-text'] : tokens.color.chat['ai-bubble-text'], fontSize: tokens.typography.fontSize.sm, maxWidth: '70%', wordBreak: 'break-word' }}>
                    {m.text}
                  </span>
                </div>
              ))}
            </div>
            {error && (
              <div style={{ padding: tokens.spacing.sm, marginBottom: tokens.spacing.sm }}>
                <span style={{ color: tokens.color.semantic.error, fontSize: tokens.typography.fontSize.sm }}>{error}</span>
                <button onClick={handleChatRetry} style={{ marginLeft: tokens.spacing.sm, padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer', fontSize: tokens.typography.fontSize.xs }}>重试</button>
              </div>
            )}
            <div style={{ display: 'flex', gap: tokens.spacing.sm }}>
              <input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSend()}
                placeholder="输入消息..."
                style={{ flex: 1, padding: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm, border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.md, outline: 'none' }}
              />
              <button onClick={handleSend} style={{ padding: `${tokens.spacing.md} ${tokens.spacing.lg}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer', fontWeight: tokens.typography.fontWeight.semibold }}>发送</button>
            </div>
          </div>
        </div>
      ) : (
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
          <div style={{ width: 280, borderRight: `1px solid ${tokens.color.neutral.border}`, overflowY: 'auto', padding: tokens.spacing.sm }}>
            {dmLoading ? (
              <p style={{ padding: tokens.spacing.md, color: tokens.color.neutral['text-secondary'], fontSize: tokens.typography.fontSize.sm }}>加载中...</p>
            ) : dmError ? (
              <div style={{ padding: tokens.spacing.md }}>
                <p style={{ color: tokens.color.semantic.error, fontSize: tokens.typography.fontSize.sm }}>{dmError}</p>
                <button onClick={loadDmThreads} style={{ marginTop: tokens.spacing.sm, padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`, ...dmLoading ? {} : { background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer', fontSize: tokens.typography.fontSize.xs } }}>重试</button>
              </div>
            ) : dmThreads.length === 0 ? (
              <div style={{ padding: tokens.spacing.md, textAlign: 'center' }}>
                <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>暂无私信</p>
                <a href="/recommendations" style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.brand.primary, textDecoration: 'none' }}>通过推荐发起私信</a>
              </div>
            ) : (
              dmThreads.map((t) => (
                <div
                  key={t.thread_id}
                  onClick={() => selectDmThread(t)}
                  style={{ padding: tokens.spacing.md, background: selectedDmThread?.thread_id === t.thread_id ? tokens.color.brand['primary-light'] : 'transparent', borderRadius: tokens.borderRadius.md, cursor: 'pointer', marginBottom: tokens.spacing.xs }}
                >
                  <div style={{ fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], fontSize: tokens.typography.fontSize.sm }}>{t.other_user_name || t.other_user_id}</div>
                  <div style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'], marginTop: tokens.spacing.xs, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{t.last_message_preview || '暂无消息'}</div>
                </div>
              ))
            )}
          </div>

          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: tokens.spacing.lg }}>
            {selectedDmThread ? (
              <>
                <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.md }}>
                  {selectedDmThread.other_user_name || selectedDmThread.other_user_id}
                </h3>
                {safetyNotice && (
                  <div style={{ padding: tokens.spacing.md, background: tokens.color.semantic['warning-bg'] ?? '#FFF3CD', borderRadius: tokens.borderRadius.md, marginBottom: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm, color: tokens.color.semantic.warning ?? '#856404' }}>
                    {safetyNotice}
                  </div>
                )}
                <div style={{ flex: 1, overflowY: 'auto', border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.lg, padding: tokens.spacing.md, marginBottom: tokens.spacing.md }}>
                  {dmMessages.length === 0 ? (
                    <p style={{ color: tokens.color.neutral['text-placeholder'], fontSize: tokens.typography.fontSize.sm, textAlign: 'center' }}>对话为空</p>
                  ) : (
                    dmMessages.map((m) => (
                      <div key={m.id} style={{ marginBottom: tokens.spacing.sm }}>
                        <span style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'] }}>{m.sender_id}</span>
                        <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-primary'] }}>{m.content}</p>
                      </div>
                    ))
                  )}
                </div>
                {dmSendError && (
                  <div style={{ padding: tokens.spacing.sm, marginBottom: tokens.spacing.sm }}>
                    <span style={{ color: tokens.color.semantic.error, fontSize: tokens.typography.fontSize.sm }}>{dmSendError}</span>
                    <button onClick={handleDmRetry} style={{ marginLeft: tokens.spacing.sm, padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer', fontSize: tokens.typography.fontSize.xs }}>重试</button>
                  </div>
                )}
                <div style={{ display: 'flex', gap: tokens.spacing.sm }}>
                  <input
                    type="text"
                    value={dmInput}
                    onChange={(e) => setDmInput(e.target.value)}
                    placeholder="输入消息..."
                    style={{ flex: 1, padding: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm, borderRadius: tokens.borderRadius.md, border: `1px solid ${tokens.color.neutral.border}` }}
                    onKeyDown={(e) => { if (e.key === 'Enter') handleDmSend(); }}
                  />
                  <button onClick={handleDmSend} style={{ padding: `${tokens.spacing.md} ${tokens.spacing.lg}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>发送</button>
                </div>
              </>
            ) : (
              <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <p style={{ color: tokens.color.neutral['text-placeholder'] }}>选择一个私信对话</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
