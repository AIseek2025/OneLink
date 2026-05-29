import { createAnalyticsEvent, AnalyticsEvent, AnalyticsContext } from '../services/analytics';
import { bffClient, setAuthToken, BffClientError } from '../services/bffClient';
import { bffUrl, API_PREFIX, BFF_BASE_URL } from '../services/config';

const mockFetch = jest.fn();
global.fetch = mockFetch;

function makeCtx(overrides?: Partial<AnalyticsContext>): AnalyticsContext {
  return {
    user_id: 'test-user',
    session_id: 'test-session',
    platform: 'ios',
    app_version: '1.0.0',
    screen: 'TestScreen',
    ...overrides,
  };
}

describe('main flow smoke test (registration -> login -> chat -> recommendation -> dm)', () => {
  beforeEach(() => {
    mockFetch.mockReset();
    setAuthToken(null);
  });

  it('registration.started and registration.completed events are well-formed', () => {
    const started: AnalyticsEvent = { event_name: 'registration.started', provider: 'email' };
    const completed: AnalyticsEvent = { event_name: 'registration.completed', user_id: 'new-user', provider: 'email' };
    const s = createAnalyticsEvent(started, makeCtx({ screen: 'RegisterScreen' }));
    const c = createAnalyticsEvent(completed, makeCtx({ screen: 'RegisterScreen', user_id: 'new-user' }));
    expect(s.event_name).toBe('registration.started');
    expect(c.event_name).toBe('registration.completed');
    expect(c.user_id).toBe('new-user');
  });

  it('login.started and login.completed events are well-formed', () => {
    const started: AnalyticsEvent = { event_name: 'login.started', provider: 'email' };
    const completed: AnalyticsEvent = { event_name: 'login.completed', user_id: 'u1', provider: 'email' };
    const s = createAnalyticsEvent(started, makeCtx({ screen: 'LoginScreen' }));
    const c = createAnalyticsEvent(completed, makeCtx({ screen: 'LoginScreen', user_id: 'u1' }));
    expect(s.event_name).toBe('login.started');
    expect(c.event_name).toBe('login.completed');
    expect(c.user_id).toBe('u1');
  });

  it('chat.message.sent and chat.message.received events are well-formed', () => {
    const sent: AnalyticsEvent = {
      event_name: 'chat.message.sent',
      conversation_id: 'conv1',
      content_type: 'text',
    };
    const received: AnalyticsEvent = {
      event_name: 'chat.message.received',
      conversation_id: 'conv1',
      response_latency_ms: 200,
    };
    const s = createAnalyticsEvent(sent, makeCtx({ screen: 'ChatScreen' }));
    const r = createAnalyticsEvent(received, makeCtx({ screen: 'ChatScreen' }));
    expect(s.event_name).toBe('chat.message.sent');
    expect(r.event_name).toBe('chat.message.received');
    expect(r.response_latency_ms).toBe(200);
  });

  it('recommendation_card_exposed and recommendation_feedback_submit events are well-formed', () => {
    const exposed: AnalyticsEvent = {
      event_name: 'recommendation_card_exposed',
      result_set_id: 'rs1',
      candidate_count: 3,
      position: 1,
    };
    const feedback: AnalyticsEvent = {
      event_name: 'recommendation_feedback_submit',
      recommendation_id: 'rec1',
      feedback_type: 'positive',
    };
    const e = createAnalyticsEvent(exposed, makeCtx({ screen: 'RecommendationsScreen' }));
    const f = createAnalyticsEvent(feedback, makeCtx({ screen: 'RecommendationsScreen' }));
    expect(e.event_name).toBe('recommendation_card_exposed');
    expect(f.event_name).toBe('recommendation_feedback_submit');
  });

  it('dm_first_message_submit event is well-formed', () => {
    const dm: AnalyticsEvent = {
      event_name: 'dm_first_message_submit',
      recommendation_id: 'rec1',
    };
    const result = createAnalyticsEvent(dm, makeCtx({ screen: 'ChatScreen' }));
    expect(result.event_name).toBe('dm_first_message_submit');
    expect(result.recommendation_id).toBe('rec1');
  });

  it('bffClient constructs correct URLs for main flow endpoints', () => {
    expect(bffUrl('/auth/register')).toBe(`${BFF_BASE_URL}${API_PREFIX}/auth/register`);
    expect(bffUrl('/auth/login')).toBe(`${BFF_BASE_URL}${API_PREFIX}/auth/login`);
    expect(bffUrl('/chat/conversations')).toBe(`${BFF_BASE_URL}${API_PREFIX}/chat/conversations`);
    expect(bffUrl('/recommendations')).toBe(`${BFF_BASE_URL}${API_PREFIX}/recommendations`);
    expect(bffUrl('/dm/threads')).toBe(`${BFF_BASE_URL}${API_PREFIX}/dm/threads`);
  });

  it('bffClient.post for registration flow sends correct payload shape', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ user_id: 'new-user', access_token: 'jwt-123', refresh_token: 'rt-456' }),
    });
    const result = await bffClient.post<{ user_id: string; access_token: string }>('/auth/register', {
      provider: 'email',
      email: 'test@example.com',
      password: 'secret',
    });
    expect(mockFetch).toHaveBeenCalledTimes(1);
    const [url, opts] = mockFetch.mock.calls[0];
    expect(url).toContain('/auth/register');
    expect(opts.method).toBe('POST');
    const body = JSON.parse(opts.body);
    expect(body.provider).toBe('email');
    expect(body.email).toBe('test@example.com');
    expect(result.user_id).toBe('new-user');
  });

  it('bffClient.post for login flow sends correct payload and sets auth token', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ user_id: 'u1', access_token: 'jwt-abc', refresh_token: 'rt-def' }),
    });
    const result = await bffClient.post<{ user_id: string; access_token: string }>('/auth/login', {
      provider: 'email',
      email: 'test@example.com',
      password: 'secret',
    });
    expect(result.access_token).toBe('jwt-abc');
    setAuthToken(result.access_token);
    const [, opts] = mockFetch.mock.calls[0];
    expect(opts.headers['Authorization']).toBeUndefined();
  });

  it('bffClient.get with auth token includes Authorization header', async () => {
    setAuthToken('jwt-abc');
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ conversations: [] }),
    });
    await bffClient.get('/chat/conversations');
    const [, opts] = mockFetch.mock.calls[0];
    expect(opts.headers['Authorization']).toBe('Bearer jwt-abc');
  });

  it('bffClient handles 401 error with BffClientError', async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 401,
      json: async () => ({ code: 'AUTH_EXPIRED', message: 'Session expired', trace_id: 't1' }),
    });
    await expect(bffClient.get('/chat/conversations')).rejects.toThrow(BffClientError);
    try {
      await bffClient.get('/chat/conversations');
    } catch (e) {
      const err = e as BffClientError;
      expect(err.status).toBe(401);
      expect(err.code).toBe('AUTH_EXPIRED');
      expect(err.traceId).toBe('t1');
    }
  });

  it('full main flow: register -> login -> chat -> recommendation -> dm analytics events form a coherent chain', () => {
    const flow: AnalyticsEvent[] = [
      { event_name: 'registration.started', provider: 'email' },
      { event_name: 'registration.completed', user_id: 'u1', provider: 'email' },
      { event_name: 'login.started', provider: 'email' },
      { event_name: 'login.completed', user_id: 'u1', provider: 'email' },
      { event_name: 'find.intent.submitted', query: 'nearby friends', query_length: 14 },
      { event_name: 'recommendation_card_exposed', result_set_id: 'rs1', candidate_count: 5, position: 1 },
      { event_name: 'recommendation_detail_view', recommendation_id: 'rec1' },
      { event_name: 'recommendation_feedback_submit', recommendation_id: 'rec1', feedback_type: 'positive' },
      { event_name: 'dm_first_message_submit', recommendation_id: 'rec1' },
      { event_name: 'chat.message.sent', conversation_id: 'conv1', content_type: 'text' },
      { event_name: 'chat.message.received', conversation_id: 'conv1', response_latency_ms: 150 },
    ];
    const screens = [
      'RegisterScreen', 'RegisterScreen',
      'LoginScreen', 'LoginScreen',
      'FindScreen', 'RecommendationsScreen',
      'RecommendationsScreen', 'RecommendationsScreen',
      'ChatScreen',
      'ChatScreen', 'ChatScreen',
    ];
    const results = flow.map((event, i) =>
      createAnalyticsEvent(event, makeCtx({ screen: screens[i], user_id: 'u1' }))
    );
    expect(results).toHaveLength(11);
    expect(results[0].event_name).toBe('registration.started');
    expect(results[results.length - 1].event_name).toBe('chat.message.received');
    for (const r of results) {
      expect(r.occurred_at).toBeDefined();
      expect(r.session_id).toBe('test-session');
    }
  });
});
