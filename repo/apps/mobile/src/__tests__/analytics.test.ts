import {
  AnalyticsEvent,
  AnalyticsContext,
  createAnalyticsEvent,
  trackEvent,
  setAnalyticsEndpoint,
  getAnalyticsContext,
  flushQueue,
  resetQueue,
} from '../services/analytics';
import { setAuthToken } from '../services/bffClient';
import AsyncStorage from '@react-native-async-storage/async-storage';

const mockFetch = jest.fn();
global.fetch = mockFetch;

describe('analytics', () => {
  beforeEach(async () => {
    mockFetch.mockReset();
    setAuthToken(null);
    resetQueue();
    setAnalyticsEndpoint('http://test-host/analytics/events');
    await AsyncStorage.clear();
  });

  it('createAnalyticsEvent merges event with context and adds occurred_at', () => {
    const event: AnalyticsEvent = {
      event_name: 'page.view',
      page_path: 'Home',
      referrer: 'Splash',
    };
    const ctx: AnalyticsContext = {
      user_id: 'u1',
      session_id: 's1',
      platform: 'ios',
      app_version: '0.3.0',
      screen: 'HomeScreen',
      trace_id: 't1',
    };
    const result = createAnalyticsEvent(event, ctx);
    expect(result.event_name).toBe('page.view');
    expect(result.page_path).toBe('Home');
    expect(result.referrer).toBe('Splash');
    expect(result.user_id).toBe('u1');
    expect(result.session_id).toBe('s1');
    expect(result.platform).toBe('ios');
    expect(result.app_version).toBe('0.3.0');
    expect(result.screen).toBe('HomeScreen');
    expect(result.trace_id).toBe('t1');
    expect(result.occurred_at).toBeDefined();
    expect(typeof result.occurred_at).toBe('string');
  });

  it('trackEvent posts to analytics endpoint with auth header when token set', async () => {
    setAuthToken('my-jwt');
    const event: AnalyticsEvent = { event_name: 'login.started', provider: 'email' };
    const ctx: AnalyticsContext = {
      user_id: null,
      session_id: 's2',
      platform: 'android',
      app_version: '0.3.0',
      screen: 'LoginScreen',
    };
    mockFetch.mockResolvedValue({ ok: true });
    await trackEvent(event, ctx);
    await flushQueue();
    expect(mockFetch).toHaveBeenCalledTimes(1);
    const [url, options] = mockFetch.mock.calls[0];
    expect(url).toBe('http://test-host/analytics/events');
    expect(options.method).toBe('POST');
    expect(options.headers['Authorization']).toBe('Bearer my-jwt');
    expect(options.headers['Content-Type']).toBe('application/json');
    const body = JSON.parse(options.body);
    const eventsPayload = body.events;
    expect(eventsPayload[0].event_name).toBe('login.started');
    expect(eventsPayload[0].provider).toBe('email');
  });

  it('trackEvent omits auth header when no token', async () => {
    const event: AnalyticsEvent = { event_name: 'page.view', page_path: 'Find' };
    const ctx: AnalyticsContext = {
      user_id: null,
      session_id: 's3',
      platform: 'ios',
      app_version: '0.3.0',
      screen: 'FindScreen',
    };
    mockFetch.mockResolvedValue({ ok: true });
    await trackEvent(event, ctx);
    await flushQueue();
    const [, options] = mockFetch.mock.calls[0];
    expect(options.headers['Authorization']).toBeUndefined();
  });

  it('trackEvent swallows transport errors without throwing', async () => {
    mockFetch.mockRejectedValue(new Error('Network down'));
    const event: AnalyticsEvent = { event_name: 'error.occurred', error_type: 'network' };
    const ctx: AnalyticsContext = {
      user_id: null,
      session_id: 's4',
      platform: 'ios',
      app_version: '0.3.0',
      screen: 'HomeScreen',
    };
    await expect(trackEvent(event, ctx)).resolves.toBeUndefined();
  });

  it('covers all major AnalyticsEvent variants without type errors', () => {
    const events: AnalyticsEvent[] = [
      { event_name: 'registration.started', provider: 'email' },
      { event_name: 'registration.completed', user_id: 'u1', provider: 'email' },
      { event_name: 'login.completed', user_id: 'u1', provider: 'phone' },
      { event_name: 'chat.message.sent', conversation_id: 'c1', content_type: 'text' },
      { event_name: 'chat.message.received', conversation_id: 'c1', response_latency_ms: 120 },
      { event_name: 'profile.confirmation.viewed', completion_rate: 0.6 },
      { event_name: 'profile.fact.confirmed', fact_type: 'hobby', fact_value: 'hiking' },
      { event_name: 'profile.fact.dismissed', fact_type: 'hobby', fact_value: 'hiking' },
      { event_name: 'find.intent.submitted', query: 'nearby', query_length: 6 },
      { event_name: 'recommendation_card_exposed', result_set_id: 'rs1', candidate_count: 5, position: 1 },
      { event_name: 'recommendation_detail_view', recommendation_id: 'rec1' },
      { event_name: 'recommendation_feedback_submit', recommendation_id: 'rec1', feedback_type: 'positive' },
      { event_name: 'dm_first_message_submit', recommendation_id: 'rec1' },
      { event_name: 'dm_first_message_approved', thread_id: 't1', recipient_user_id: 'u2' },
      { event_name: 'report_submit', target_type: 'user', target_id: 'u3', reason: 'spam' },
      { event_name: 'locale_setting_save' },
      { event_name: 'data_export_request' },
      { event_name: 'data_correction_request' },
      { event_name: 'data_delete_request' },
      { event_name: 'error.occurred', error_type: 'auth', error_code: 'E401', error_message: 'expired', http_status: 401 },
    ];
    const ctx: AnalyticsContext = {
      user_id: 'u1',
      session_id: 's5',
      platform: 'ios',
      app_version: '0.3.0',
      screen: 'TestScreen',
    };
    for (const event of events) {
      const result = createAnalyticsEvent(event, ctx);
      expect(result.event_name).toBeDefined();
      expect(result.occurred_at).toBeDefined();
    }
  });

  it('getAnalyticsContext returns valid context with overrides', async () => {
    const ctx = await getAnalyticsContext({ user_id: 'u1', screen: 'MeScreen' });
    expect(ctx.session_id).toBeDefined();
    expect(typeof ctx.session_id).toBe('string');
    expect(ctx.platform).toMatch(/^(ios|android)$/);
    expect(ctx.app_version).toBe('0.3.0');
    expect(ctx.user_id).toBe('u1');
    expect(ctx.screen).toBe('MeScreen');
  });

  it('getAnalyticsContext falls back when crypto.randomUUID is unavailable', async () => {
    const originalCrypto = global.crypto;
    await AsyncStorage.removeItem('onelink_analytics_session_id');
    Object.defineProperty(global, 'crypto', { value: undefined, configurable: true });

    try {
      const ctx = await getAnalyticsContext({ screen: 'LoginScreen' });
      expect(ctx.session_id).toMatch(/^session-/);
      expect(ctx.app_version).toBe('0.3.0');
    } finally {
      Object.defineProperty(global, 'crypto', { value: originalCrypto, configurable: true });
    }
  });
});
