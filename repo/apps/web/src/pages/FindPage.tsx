import { useEffect, useState } from 'react';
import { submitFindIntent, fetchFindRequestDetail, submitClarification } from '../api/client';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';

type FindState = 'idle' | 'submitting' | 'submitted' | 'clarifying' | 'clarified' | 'error' | 'empty_result';

export function FindPage() {
  const [query, setQuery] = useState('');
  const [intentTags] = useState('');
  const [findState, setFindState] = useState<FindState>('idle');
  const [findRequestId, setFindRequestId] = useState('');
  const [clarificationQuestions, setClarificationQuestions] = useState<{ question_id: string; question_text: string }[]>([]);
  const [clarificationAnswers, setClarificationAnswers] = useState<Record<string, string>>({});
  const [resultSummary, setResultSummary] = useState('');
  const [error, setError] = useState('');

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    trackEvent({ event_name: 'page.view', page_path: '/find' }, getAnalyticsContext({ screen: '/find' }));
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;
    setError('');
    setFindState('submitting');
    try {
      const tags = intentTags.trim() ? intentTags.split(',').map((t) => t.trim()).filter(Boolean) : [];
      const res = await submitFindIntent(query, tags);
      const requestId = res.find_request_id ?? res.request_id;
      setFindRequestId(requestId);
      const ctx = getAnalyticsContext({ screen: '/find' });
      trackEvent({ event_name: 'find.intent.submitted', user_id: ctx.user_id ?? '', query, query_length: query.length }, ctx);

      const detail = await fetchFindRequestDetail(requestId);
      const state = detail.state ?? detail.status;

      if (state === 'clarification_needed' || state === 'draft') {
        const questions = detail.clarification_questions ?? detail.questions ?? [];
        if (questions.length > 0) {
          setClarificationQuestions(questions);
          setFindState('clarifying');
        } else {
          setResultSummary(detail.summary ?? detail.result_summary ?? '请求已提交，等待匹配结果');
          setFindState('submitted');
        }
      } else if (state === 'completed' || state === 'results_ready') {
        setResultSummary(detail.summary ?? detail.result_summary ?? '找到匹配结果');
        setFindState('submitted');
      } else if (state === 'empty_result' || state === 'no_match') {
        setResultSummary(detail.summary ?? '未找到匹配，建议调整描述后重试');
        setFindState('empty_result');
      } else {
        setResultSummary(detail.summary ?? '请求已提交');
        setFindState('submitted');
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Submit failed';
      setError(msg);
      setFindState('error');
      trackEvent({ event_name: 'error.occurred', error_type: 'network', error_message: msg }, getAnalyticsContext({ screen: '/find' }));
    }
  };

  const handleClarificationSubmit = async () => {
    setError('');
    try {
      await submitClarification(findRequestId, clarificationAnswers);
      const detail = await fetchFindRequestDetail(findRequestId);
      const state = detail.state ?? detail.status;
      setResultSummary(detail.summary ?? detail.result_summary ?? '澄清已提交，等待匹配结果');
      if (state === 'empty_result' || state === 'no_match') {
        setFindState('empty_result');
      } else {
        setFindState('clarified');
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Clarification failed';
      setError(msg);
      setFindState('error');
    }
  };

  const resetForm = () => {
    setQuery('');
    setFindState('idle');
    setFindRequestId('');
    setClarificationQuestions([]);
    setClarificationAnswers({});
    setResultSummary('');
    setError('');
  };

  return (
    <div style={{ maxWidth: 640, margin: '0 auto', padding: tokens.spacing.xl }}>
      <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'] }}>找人</h2>

      {findState === 'idle' && (
        <form onSubmit={handleSubmit}>
          <p style={{ color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.md }}>描述你想找什么样的人</p>
          <textarea
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="例如：希望认识对 AI 创业感兴趣的投资人"
            rows={4}
            style={{ width: '100%', padding: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm, borderRadius: tokens.borderRadius.lg, border: `1px solid ${tokens.color.neutral.border}`, outline: 'none' }}
          />
          <button type="submit" style={{ marginTop: tokens.spacing.sm, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer', fontWeight: tokens.typography.fontWeight.semibold }}>
            提交找人意图
          </button>
        </form>
      )}

      {findState === 'submitting' && (
        <div style={{ padding: tokens.spacing.xl, textAlign: 'center', color: tokens.color.neutral['text-secondary'] }}>
          提交中...
        </div>
      )}

      {findState === 'clarifying' && (
        <div style={{ padding: tokens.spacing.lg, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg }}>
          <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.md }}>澄清问题</h3>
          <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.md }}>请回答以下问题以帮助精准匹配</p>
          {clarificationQuestions.map((q) => (
            <div key={q.question_id} style={{ marginBottom: tokens.spacing.md }}>
              <p style={{ fontSize: tokens.typography.fontSize.sm, fontWeight: tokens.typography.fontWeight.medium, color: tokens.color.neutral['text-primary'] }}>{q.question_text}</p>
              <input
                type="text"
                value={clarificationAnswers[q.question_id] ?? ''}
                onChange={(e) => setClarificationAnswers({ ...clarificationAnswers, [q.question_id]: e.target.value })}
                style={{ width: '100%', padding: tokens.spacing.sm, fontSize: tokens.typography.fontSize.sm, borderRadius: tokens.borderRadius.md, border: `1px solid ${tokens.color.neutral.border}` }}
              />
            </div>
          ))}
          <button onClick={handleClarificationSubmit} style={{ marginTop: tokens.spacing.md, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>
            提交回答
          </button>
          {error && <p style={{ color: tokens.color.semantic.error, fontSize: tokens.typography.fontSize.sm }}>{error}</p>}
        </div>
      )}

      {findState === 'clarified' && (
        <div style={{ padding: tokens.spacing.lg, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg }}>
          <p style={{ color: tokens.color.neutral['text-primary'], fontWeight: tokens.typography.fontWeight.medium }}>澄清回答已提交</p>
          <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>{resultSummary}</p>
          <a href="/recommendations" style={{ display: 'inline-block', marginTop: tokens.spacing.md, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', borderRadius: tokens.borderRadius.md, textDecoration: 'none' }}>
            查看推荐结果
          </a>
          <button onClick={resetForm} style={{ marginLeft: tokens.spacing.sm, marginTop: tokens.spacing.md, padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`, background: tokens.color.neutral.surface, color: tokens.color.neutral['text-secondary'], border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>
            继续找人
          </button>
        </div>
      )}

      {(findState === 'submitted') && (
        <div style={{ padding: tokens.spacing.lg, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg }}>
          <p style={{ color: tokens.color.neutral['text-primary'], fontWeight: tokens.typography.fontWeight.medium }}>意图已提交</p>
          <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>请求 ID: {findRequestId}</p>
          <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>{resultSummary}</p>
          <a href="/recommendations" style={{ display: 'inline-block', marginTop: tokens.spacing.md, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', borderRadius: tokens.borderRadius.md, textDecoration: 'none' }}>
            查看推荐结果
          </a>
          <button onClick={resetForm} style={{ marginLeft: tokens.spacing.sm, marginTop: tokens.spacing.md, padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`, background: tokens.color.neutral.surface, color: tokens.color.neutral['text-secondary'], border: `1px solid ${tokens.color.neutral.border}`, borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>
            继续找人
          </button>
        </div>
      )}

      {findState === 'empty_result' && (
        <div style={{ padding: tokens.spacing.lg, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg }}>
          <p style={{ color: tokens.color.neutral['text-primary'], fontWeight: tokens.typography.fontWeight.medium }}>未找到匹配</p>
          <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>{resultSummary}</p>
          <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-placeholder'], marginTop: tokens.spacing.sm }}>建议调整描述或增加关键词后重试</p>
          <button onClick={resetForm} style={{ marginTop: tokens.spacing.md, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>
            重新找人
          </button>
        </div>
      )}

      {findState === 'error' && (
        <div style={{ padding: tokens.spacing.lg, background: tokens.color.semantic['error-bg'], borderRadius: tokens.borderRadius.lg }}>
          <p style={{ color: tokens.color.semantic.error }}>{error}</p>
          <button onClick={resetForm} style={{ marginTop: tokens.spacing.sm, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', border: 'none', borderRadius: tokens.borderRadius.md, cursor: 'pointer' }}>
            重试
          </button>
        </div>
      )}
    </div>
  );
}