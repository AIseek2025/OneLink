import { useEffect, useState } from 'react';
import { fetchOnboarding, answerQuestion } from '../api/client';
import { tokens } from '../design-tokens';
import { trackEvent, getAnalyticsContext } from '../analytics';
import { OlNavBar } from '../components/OlNavBar';
import { OlButton } from '../components/OlButton';
import { OlErrorBanner, OlLoading } from '../components/OlErrorBanner';

interface PendingQuestion {
  delivery_id: string;
  variant_id?: string;
  question_text: string;
  question_style: 'single_choice' | 'multi_choice' | 'text_input';
  options?: string[];
  requirement_tier?: 'starter_required' | 'profile_required' | 'optional';
}

interface QuestionnaireProgress {
  starter_required_count: number;
  starter_required_total: number;
  profile_required_count: number;
  profile_required_total: number;
  optional_count: number;
  can_proceed_to_find: boolean;
  degraded?: boolean;
}

interface OnboardingData {
  user: { user_id: string; status: string; primary_region: string; primary_language: string };
  pending_questions: PendingQuestion[];
  progress: QuestionnaireProgress;
}

export function QuestionnairePage() {
  const [data, setData] = useState<OnboardingData | null>(null);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [answers, setAnswers] = useState<Record<string, string | string[]>>({});
  const [textInput, setTextInput] = useState('');
  const [multiSelections, setMultiSelections] = useState<string[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');
  const [completed, setCompleted] = useState(false);

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { window.location.href = '/login'; return; }
    fetchOnboarding()
      .then((d) => {
        setData(d);
        trackEvent(
          { event_name: 'page.view', page_path: '/questionnaire' },
          getAnalyticsContext({ screen: '/questionnaire', user_id: d.user?.user_id ?? null }),
        );
      })
      .catch((e) => {
        setError(e.message);
        trackEvent(
          { event_name: 'error.occurred', error_type: 'network', error_message: e.message },
          getAnalyticsContext({ screen: '/questionnaire' }),
        );
      });
  }, []);

  const currentQuestion = data?.pending_questions[currentIndex] ?? null;

  const handleSingleChoice = (option: string) => {
    if (!currentQuestion) return;
    setAnswers((prev) => ({ ...prev, [currentQuestion.delivery_id]: option }));
  };

  const handleMultiToggle = (option: string) => {
    setMultiSelections((prev) =>
      prev.includes(option) ? prev.filter((o) => o !== option) : [...prev, option],
    );
  };

  const handleSubmitAnswer = async () => {
    if (!currentQuestion || !data) return;
    setSubmitting(true);
    setError('');

    let answerValue: string | string[];
    if (currentQuestion.question_style === 'text_input') {
      answerValue = textInput;
    } else if (currentQuestion.question_style === 'multi_choice') {
      answerValue = multiSelections;
    } else {
      answerValue = answers[currentQuestion.delivery_id] as string;
    }

    if (!answerValue || (Array.isArray(answerValue) && answerValue.length === 0)) {
      setError('请选择或输入答案');
      setSubmitting(false);
      return;
    }

    try {
      await answerQuestion(
        currentQuestion.delivery_id,
        currentQuestion.variant_id,
        answerValue,
      );
      trackEvent(
        { event_name: 'profile.fact.confirmed', user_id: data.user?.user_id ?? '', fact_type: currentQuestion.question_text, fact_value: JSON.stringify(answerValue) },
        getAnalyticsContext({ screen: '/questionnaire', user_id: data.user?.user_id ?? null }),
      );

      if (currentIndex < data.pending_questions.length - 1) {
        setCurrentIndex((prev) => prev + 1);
        setTextInput('');
        setMultiSelections([]);
      } else {
        setCompleted(true);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Submit failed';
      setError(msg);
      trackEvent(
        { event_name: 'error.occurred', error_type: 'network', error_message: msg },
        getAnalyticsContext({ screen: '/questionnaire' }),
      );
    } finally {
      setSubmitting(false);
    }
  };

  const navItems = [
    { label: '首页', href: '/' },
    { label: '聊天', href: '/chat' },
    { label: '画像', href: '/profile' },
    { label: '找人', href: '/find' },
    { label: '问卷', href: '/questionnaire', active: true },
  ];

  const handleLogout = () => {
    localStorage.removeItem('onelink_token');
    window.location.href = '/login';
  };

  if (error && !data) {
    return (
      <div>
        <OlNavBar items={navItems} onLogout={handleLogout} />
        <div style={{ maxWidth: 640, margin: '0 auto', padding: tokens.spacing.xl }}>
          <OlErrorBanner message={error} onDismiss={() => setError('')} />
        </div>
      </div>
    );
  }

  if (!data) {
    return (
      <div>
        <OlNavBar items={navItems} onLogout={handleLogout} />
        <OlLoading text="加载问卷..." />
      </div>
    );
  }

  if (completed) {
    return (
      <div>
        <OlNavBar items={navItems} onLogout={handleLogout} />
        <div style={{ maxWidth: 640, margin: '0 auto', padding: tokens.spacing.xl }}>
          <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.lg }}>问卷完成</h2>
          <p style={{ color: tokens.color.neutral['text-secondary'], marginBottom: tokens.spacing.lg }}>
            感谢完成问卷！你的画像信息已更新。
          </p>
          {data.progress.can_proceed_to_find && (
            <a href="/find" style={{ display: 'inline-block', padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.brand.primary, color: '#FFFFFF', borderRadius: tokens.borderRadius.md, textDecoration: 'none', fontWeight: tokens.typography.fontWeight.semibold }}>
              开始找人
            </a>
          )}
          <a href="/" style={{ display: 'inline-block', marginLeft: tokens.spacing.md, padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.md, textDecoration: 'none', color: tokens.color.neutral['text-primary'] }}>
            返回首页
          </a>
        </div>
      </div>
    );
  }

  const progress = data.progress;
  const totalQuestions = data.pending_questions.length;

  return (
    <div>
      <OlNavBar items={navItems} onLogout={handleLogout} />
      <div style={{ maxWidth: 640, margin: '0 auto', padding: tokens.spacing.xl }}>
        <h2 style={{ fontSize: tokens.typography.fontSize.xl, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.sm }}>完善画像</h2>

        {progress && (
          <div style={{ marginBottom: tokens.spacing.lg, padding: tokens.spacing.md, background: tokens.color.neutral.surface, borderRadius: tokens.borderRadius.lg }}>
            <div style={{ display: 'flex', gap: tokens.spacing.lg, fontSize: tokens.typography.fontSize.sm, color: tokens.color.neutral['text-secondary'] }}>
              <span>必填: {progress.starter_required_count}/{progress.starter_required_total}</span>
              <span>画像: {progress.profile_required_count}/{progress.profile_required_total}</span>
            </div>
            <div style={{ marginTop: tokens.spacing.sm, height: 4, background: tokens.color.neutral.border, borderRadius: tokens.borderRadius.full, overflow: 'hidden' }}>
              <div style={{ height: '100%', width: `${((currentIndex + 1) / totalQuestions) * 100}%`, background: tokens.color.brand.primary, borderRadius: tokens.borderRadius.full, transition: `width var(--ol-motion-normal, 250ms) ease-in-out` }} />
            </div>
            <p style={{ marginTop: tokens.spacing.xs, fontSize: tokens.typography.fontSize.xs, color: tokens.color.neutral['text-secondary'] }}>
              {currentIndex + 1} / {totalQuestions}
            </p>
          </div>
        )}

        {currentQuestion && (
          <div style={{ marginBottom: tokens.spacing.xl }}>
            <p style={{ fontSize: tokens.typography.fontSize.lg, fontWeight: tokens.typography.fontWeight.semibold, color: tokens.color.neutral['text-primary'], marginBottom: tokens.spacing.lg }}>
              {currentQuestion.question_text}
            </p>

            {currentQuestion.question_style === 'single_choice' && currentQuestion.options && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.sm }}>
                {currentQuestion.options.map((option) => (
                  <button
                    key={option}
                    onClick={() => handleSingleChoice(option)}
                    style={{
                      padding: `${tokens.spacing.md} ${tokens.spacing.lg}`,
                      textAlign: 'left',
                      background: answers[currentQuestion.delivery_id] === option ? tokens.color.brand['primary-light'] : tokens.color.neutral.bg,
                      border: `1px solid ${answers[currentQuestion.delivery_id] === option ? tokens.color.brand.primary : tokens.color.neutral.border}`,
                      borderRadius: tokens.borderRadius.md,
                      cursor: 'pointer',
                      fontSize: tokens.typography.fontSize.sm,
                      color: answers[currentQuestion.delivery_id] === option ? tokens.color.brand.primary : tokens.color.neutral['text-primary'],
                      fontWeight: answers[currentQuestion.delivery_id] === option ? tokens.typography.fontWeight.semibold : tokens.typography.fontWeight.normal,
                      transition: `all var(--ol-motion-fast, 150ms) ease-in-out`,
                    }}
                  >
                    {option}
                  </button>
                ))}
              </div>
            )}

            {currentQuestion.question_style === 'multi_choice' && currentQuestion.options && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.sm }}>
                {currentQuestion.options.map((option) => (
                  <button
                    key={option}
                    onClick={() => handleMultiToggle(option)}
                    style={{
                      padding: `${tokens.spacing.md} ${tokens.spacing.lg}`,
                      textAlign: 'left',
                      background: multiSelections.includes(option) ? tokens.color.brand['primary-light'] : tokens.color.neutral.bg,
                      border: `1px solid ${multiSelections.includes(option) ? tokens.color.brand.primary : tokens.color.neutral.border}`,
                      borderRadius: tokens.borderRadius.md,
                      cursor: 'pointer',
                      fontSize: tokens.typography.fontSize.sm,
                      color: multiSelections.includes(option) ? tokens.color.brand.primary : tokens.color.neutral['text-primary'],
                      fontWeight: multiSelections.includes(option) ? tokens.typography.fontWeight.semibold : tokens.typography.fontWeight.normal,
                      transition: `all var(--ol-motion-fast, 150ms) ease-in-out`,
                    }}
                  >
                    {option}
                  </button>
                ))}
              </div>
            )}

            {currentQuestion.question_style === 'text_input' && (
              <textarea
                value={textInput}
                onChange={(e) => setTextInput(e.target.value)}
                placeholder="输入你的回答..."
                rows={3}
                style={{
                  width: '100%',
                  padding: tokens.spacing.md,
                  fontSize: tokens.typography.fontSize.sm,
                  borderRadius: tokens.borderRadius.lg,
                  border: `1px solid ${tokens.color.neutral.border}`,
                  outline: 'none',
                  resize: 'vertical',
                  fontFamily: tokens.typography.fontFamily.base,
                }}
              />
            )}
          </div>
        )}

        {error && <OlErrorBanner message={error} onDismiss={() => setError('')} />}

        <div style={{ display: 'flex', gap: tokens.spacing.md, justifyContent: 'flex-end' }}>
          {currentIndex > 0 && (
            <OlButton variant="secondary" onClick={() => { setCurrentIndex((prev) => prev - 1); setTextInput(''); setMultiSelections([]); }}>
              上一题
            </OlButton>
          )}
          <OlButton variant="primary" onClick={handleSubmitAnswer} loading={submitting}>
            {currentIndex < totalQuestions - 1 ? '下一题' : '完成'}
          </OlButton>
        </div>
      </div>
    </div>
  );
}
