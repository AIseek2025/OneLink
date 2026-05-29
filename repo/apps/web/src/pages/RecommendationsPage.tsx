import { useEffect, useState } from 'react';
import { fetchRecommendationList, fetchRecommendationDetail, submitRecommendationFeedback, createDmDraft, submitDmFirstMessage } from '../api/client';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';

interface Candidate {
  recommendation_id: string;
  user_id: string;
  display_name: string;
  headline: string;
  city_level_location: string;
  match_score: number;
  interest_tags: string[];
  explanation?: string;
}

interface RecDetail {
  recommendation_id: string;
  user_id: string;
  display_name: string;
  headline: string;
  city_level_location: string;
  match_score: number;
  interest_tags: string[];
  explanation: string;
  profile_summary: string;
  shared_interests: string[];
}

type PageState = 'list' | 'detail' | 'feedback' | 'connect';
type FeedbackType = 'positive' | 'neutral' | 'negative';

export function RecommendationsPage() {
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [pageState, setPageState] = useState<PageState>('list');
  const [selectedRecId, setSelectedRecId] = useState('');
  const [detail, setDetail] = useState<RecDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [feedbackType, setFeedbackType] = useState<FeedbackType>('neutral');
  const [feedbackComment, setFeedbackComment] = useState('');
  const [feedbackSubmitting, setFeedbackSubmitting] = useState(false);
  const [feedbackSuccess, setFeedbackSuccess] = useState('');
  const [connectMessage, setConnectMessage] = useState('');
  const [connectSending, setConnectSending] = useState(false);
  const [connectSuccess, setConnectSuccess] = useState('');
  const [connectError, setConnectError] = useState('');

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    trackEvent({ event_name: 'page.view', page_path: '/recommendations' }, getAnalyticsContext({ screen: '/recommendations' }));
    loadResults();
  }, []);

  const loadResults = async () => {
    setLoading(true);
    setError('');
    try {
      const res = await fetchRecommendationList();
const items: Candidate[] = res.candidates ?? res.items ?? res.recommendations ?? [];
      setCandidates(items);
      if (items.length > 0) {
        const ctx = getAnalyticsContext({ screen: '/recommendations' });
        items.forEach((_, i) => {
          trackEvent(
            { event_name: 'recommendation.exposed', user_id: ctx.user_id ?? '', result_set_id: res.find_request_id ?? '', candidate_count: items.length, position: i },
            ctx,
          );
        });
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to load recommendations';
      setError(msg);
      trackEvent({ event_name: 'error.occurred', error_type: 'network', error_message: msg }, getAnalyticsContext({ screen: '/recommendations' }));
    } finally {
      setLoading(false);
    }
  };

  const openDetail = async (recId: string) => {
    setSelectedRecId(recId);
    setDetailLoading(true);
    setDetail(null);
    setPageState('detail');
    try {
      const d = await fetchRecommendationDetail(recId);
      setDetail(d);
      trackEvent({ event_name: 'recommendation.detail.viewed', recommendation_id: recId }, getAnalyticsContext({ screen: '/recommendations' }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load detail');
    } finally {
      setDetailLoading(false);
    }
  };

  const handleFeedback = async () => {
    setFeedbackSubmitting(true);
    setFeedbackSuccess('');
    setError('');
    try {
      await submitRecommendationFeedback(selectedRecId, feedbackType, feedbackComment || undefined);
      setFeedbackSuccess('反馈已提交');
      trackEvent({ event_name: 'recommendation.feedback.submitted', recommendation_id: selectedRecId, feedback_type: feedbackType }, getAnalyticsContext({ screen: '/recommendations' }));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Feedback failed');
    } finally {
      setFeedbackSubmitting(false);
    }
  };

  const handleConnect = async () => {
    if (!connectMessage.trim()) return;
    setConnectSending(true);
    setConnectError('');
    setConnectSuccess('');
    try {
      const draft = await createDmDraft(selectedRecId, connectMessage.trim());
      if (draft.thread_id) {
        await submitDmFirstMessage(draft.thread_id, connectMessage.trim());
      }
      setConnectSuccess('私信已发送');
      setConnectMessage('');
      trackEvent({ event_name: 'dm.first_message.sent', recommendation_id: selectedRecId }, getAnalyticsContext({ screen: '/recommendations' }));
    } catch (e) {
      setConnectError(e instanceof Error ? e.message : 'Failed to send message');
    } finally {
      setConnectSending(false);
    }
  };

  const resetToDetail = () => {
    setPageState('detail');
    setFeedbackType('neutral');
    setFeedbackComment('');
    setFeedbackSuccess('');
    setConnectMessage('');
    setConnectSuccess('');
    setConnectError('');
  };

  const cardStyle: React.CSSProperties = {
    padding: tokens.spacing.lg,
    background: tokens.color.neutral.surface,
    borderRadius: tokens.borderRadius.lg,
    border: `1px solid ${tokens.color.neutral.border}`,
    marginBottom: tokens.spacing.md,
    cursor: 'pointer',
  };

  const btnPrimary: React.CSSProperties = {
    padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`,
    background: tokens.color.brand.primary,
    color: '#FFFFFF',
    border: 'none',
    borderRadius: tokens.borderRadius.md,
    cursor: 'pointer',
    fontSize: tokens.typography.fontSize.sm,
  };

  const btnSecondary: React.CSSProperties = {
    ...btnPrimary,
    background: tokens.color.neutral.surface,
    color: tokens.color.neutral['text-primary'],
    border: `1px solid ${tokens.color.neutral.border}`,
  };

  return (
    <div style={{ maxWidth: 720, margin: '0 auto', padding: tokens.spacing.xl }}>
      <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.lg }}>推荐</h2>

      {error && (
        <div style={{ padding: tokens.spacing.md, background: tokens.color.semantic['error-bg'], borderRadius: tokens.borderRadius.md, color: tokens.color.semantic.error, marginBottom: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm }}>
          {error}
          <button onClick={() => { setError(''); loadResults(); }} style={{ marginLeft: tokens.spacing.md, ...btnPrimary }}>重试</button>
        </div>
      )}

      {pageState === 'list' && (
        loading ? (
          <div style={{ padding: tokens.spacing.xl, textAlign: 'center', color: tokens.color.neutral['text-secondary'] }}>加载中...</div>
        ) : candidates.length === 0 ? (
          <div style={{ padding: tokens.spacing.xl, textAlign: 'center' }}>
            <p style={{ fontSize: tokens.typography.fontSize.lg, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.md }}>暂无推荐</p>
            <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-placeholder'] }}>完善你的画像或提交更多找人意图，系统将为你生成更精准的推荐</p>
            <a href="/find" style={{ display: 'inline-block', marginTop: tokens.spacing.lg, ...btnPrimary, textDecoration: 'none' }}>去找人</a>
          </div>
        ) : (
          <div>
            {candidates.map((c) => (
              <div key={c.recommendation_id ?? c.user_id} style={cardStyle} onClick={() => openDetail(c.recommendation_id ?? c.user_id)}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: tokens.spacing.sm }}>
                  <span style={{ fontSize: tokens.typography.fontSize.lg, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'] }}>{c.display_name}</span>
                  <span style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'] }}>匹配度 {Math.round(c.match_score * 100)}%</span>
                </div>
                <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.xs }}>{c.headline}</p>
                <p style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-placeholder'] }}>{c.city_level_location}</p>
                {c.interest_tags && c.interest_tags.length > 0 && (
                  <div style={{ display: 'flex', gap: tokens.spacing.xs, marginTop: tokens.spacing.sm, flexWrap: 'wrap' }}>
                    {c.interest_tags.map((tag) => (
                      <span key={tag} style={{ padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`, background: tokens.color.brand['primary-light'], color: tokens.color.brand.primary, borderRadius: tokens.borderRadius.sm, fontSize: tokens.typography.fontSize.xs }}>
                        {tag}
                      </span>
                    ))}
                  </div>
                )}
                {c.explanation && (
                  <p style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'], marginTop: tokens.spacing.sm, fontStyle: 'italic' }}>{c.explanation}</p>
                )}
              </div>
            ))}
          </div>
        )
      )}

      {pageState === 'detail' && (
        <div>
          <button onClick={() => setPageState('list')} style={{ ...btnSecondary, marginBottom: tokens.spacing.md }}>← 返回列表</button>
          {detailLoading ? (
            <div style={{ padding: tokens.spacing.xl, textAlign: 'center', color: tokens.color.neutral['text-secondary'] }}>加载详情...</div>
          ) : detail ? (
            <div style={{ padding: tokens.spacing.lg, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg, border: `1px solid ${tokens.color.neutral.border}` }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: tokens.spacing.md }}>
                <span style={{ fontSize: tokens.typography.fontSize.xl, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'] }}>{detail.display_name}</span>
                <span style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>匹配度 {Math.round(detail.match_score * 100)}%</span>
              </div>
              <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.sm }}>{detail.headline}</p>
              <p style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-placeholder'], marginBottom: tokens.spacing.md }}>{detail.city_level_location}</p>

              {detail.explanation && (
                <div style={{ padding: tokens.spacing.md, background: tokens.color.semantic['info-bg'] ?? '#E8F4FD', borderRadius: tokens.borderRadius.md, marginBottom: tokens.spacing.md }}>
                  <p style={{ fontSize: tokens.typography.fontSize.sm, fontWeight: tokens.typography.fontWeight.medium, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.xs }}>匹配解释</p>
                  <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>{detail.explanation}</p>
                </div>
              )}

              {detail.profile_summary && (
                <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.md }}>{detail.profile_summary}</p>
              )}

              {detail.shared_interests && detail.shared_interests.length > 0 && (
                <div style={{ marginBottom: tokens.spacing.md }}>
                  <p style={{ fontSize: tokens.typography.fontSize.xs, fontWeight: tokens.typography.fontWeight.medium, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.xs }}>共同兴趣</p>
                  <div style={{ display: 'flex', gap: tokens.spacing.xs, flexWrap: 'wrap' }}>
                    {detail.shared_interests.map((tag) => (
                      <span key={tag} style={{ padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`, background: tokens.color.brand['primary-light'], color: tokens.color.brand.primary, borderRadius: tokens.borderRadius.sm, fontSize: tokens.typography.fontSize.xs }}>{tag}</span>
                    ))}
                  </div>
                </div>
              )}

              {detail.interest_tags && detail.interest_tags.length > 0 && (
                <div style={{ marginBottom: tokens.spacing.md }}>
                  <p style={{ fontSize: tokens.typography.fontSize.xs, fontWeight: tokens.typography.fontWeight.medium, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.xs }}>兴趣标签</p>
                  <div style={{ display: 'flex', gap: tokens.spacing.xs, flexWrap: 'wrap' }}>
                    {detail.interest_tags.map((tag) => (
                      <span key={tag} style={{ padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`, background: tokens.color.neutral['bg-secondary'] ?? '#F0F0F0', color: tokens.color.neutral['text-secondary'], borderRadius: tokens.borderRadius.sm, fontSize: tokens.typography.fontSize.xs }}>{tag}</span>
                    ))}
                  </div>
                </div>
              )}

              <div style={{ display: 'flex', gap: tokens.spacing.sm, marginTop: tokens.spacing.lg }}>
                <button onClick={() => { setPageState('feedback'); setFeedbackSuccess(''); }} style={btnPrimary}>反馈</button>
                <button onClick={() => { setPageState('connect'); setConnectSuccess(''); setConnectError(''); }} style={{ ...btnPrimary, background: tokens.color.semantic.success ?? '#28A745' }}>发起私信</button>
              </div>
            </div>
          ) : (
            <p style={{ color: tokens.color.neutral['text-secondary'] }}>无法加载详情</p>
          )}
        </div>
      )}

      {pageState === 'feedback' && (
        <div>
          <button onClick={resetToDetail} style={{ ...btnSecondary, marginBottom: tokens.spacing.md }}>← 返回详情</button>
          <div style={{ padding: tokens.spacing.lg, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg, border: `1px solid ${tokens.color.neutral.border}` }}>
            <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.md }}>推荐反馈</h3>
            <div style={{ display: 'flex', gap: tokens.spacing.sm, marginBottom: tokens.spacing.md }}>
              {(['positive', 'neutral', 'negative'] as FeedbackType[]).map((ft) => (
                <button
                  key={ft}
                  onClick={() => setFeedbackType(ft)}
                  style={{
                    ...btnSecondary,
                    background: feedbackType === ft ? tokens.color.brand.primary : tokens.color.neutral.surface,
                    color: feedbackType === ft ? '#FFFFFF' : tokens.color.neutral['text-primary'],
                    border: `1px solid ${feedbackType === ft ? tokens.color.brand.primary : tokens.color.neutral.border}`,
                  }}
                >
                  {ft === 'positive' ? '满意' : ft === 'neutral' ? '一般' : '不满意'}
                </button>
              ))}
            </div>
            <textarea
              value={feedbackComment}
              onChange={(e) => setFeedbackComment(e.target.value)}
              placeholder="补充说明（可选）"
              rows={3}
              style={{ width: '100%', padding: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm, borderRadius: tokens.borderRadius.lg, border: `1px solid ${tokens.color.neutral.border}`, marginBottom: tokens.spacing.md }}
            />
            {feedbackSuccess && <p style={{ color: tokens.color.semantic.success ?? '#28A745', fontSize: tokens.typography.fontSize.sm, marginBottom: tokens.spacing.md }}>{feedbackSuccess}</p>}
            <button onClick={handleFeedback} disabled={feedbackSubmitting} style={{ ...btnPrimary, opacity: feedbackSubmitting ? 0.6 : 1, cursor: feedbackSubmitting ? 'not-allowed' : 'pointer' }}>
              {feedbackSubmitting ? '提交中...' : '提交反馈'}
            </button>
          </div>
        </div>
      )}

      {pageState === 'connect' && (
        <div>
          <button onClick={resetToDetail} style={{ ...btnSecondary, marginBottom: tokens.spacing.md }}>← 返回详情</button>
          <div style={{ padding: tokens.spacing.lg, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg, border: `1px solid ${tokens.color.neutral.border}` }}>
            <h3 style={{ fontSize: tokens.typography.fontSize.base, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.md }}>发起首条私信</h3>
            <p style={{ fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.md }}>向 {detail?.display_name ?? '对方'} 发送第一条消息</p>
            <textarea
              value={connectMessage}
              onChange={(e) => setConnectMessage(e.target.value)}
              placeholder="写一段开场白..."
              rows={4}
              style={{ width: '100%', padding: tokens.spacing.md, fontSize: tokens.typography.fontSize.sm, borderRadius: tokens.borderRadius.lg, border: `1px solid ${tokens.color.neutral.border}`, marginBottom: tokens.spacing.md }}
            />
            {connectError && <p style={{ color: tokens.color.semantic.error, fontSize: tokens.typography.fontSize.sm, marginBottom: tokens.spacing.md }}>{connectError}</p>}
            {connectSuccess ? (
              <div>
                <p style={{ color: tokens.color.semantic.success ?? '#28A745', fontSize: tokens.typography.fontSize.sm, marginBottom: tokens.spacing.md }}>{connectSuccess}</p>
                <a href="/dm" style={{ ...btnPrimary, textDecoration: 'none', display: 'inline-block' }}>查看私信</a>
              </div>
            ) : (
              <button onClick={handleConnect} disabled={connectSending || !connectMessage.trim()} style={{ ...btnPrimary, opacity: (connectSending || !connectMessage.trim()) ? 0.6 : 1, cursor: (connectSending || !connectMessage.trim()) ? 'not-allowed' : 'pointer' }}>
                {connectSending ? '发送中...' : '发送私信'}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
