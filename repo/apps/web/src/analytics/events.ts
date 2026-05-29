export type AnalyticsEvent =
  | { event_name: 'page.view'; page_path: string; page_name?: string; referrer?: string }
  | { event_name: 'registration.started'; provider?: string }
  | { event_name: 'registration.completed'; user_id: string; provider: string }
  | { event_name: 'login.started'; provider?: string }
  | { event_name: 'login.completed'; user_id: string; provider: string }
  | { event_name: 'chat.message.sent'; user_id?: string; conversation_id: string; content_type: 'text' | 'image' | 'voice' }
  | { event_name: 'chat.message.received'; user_id?: string; conversation_id: string; response_latency_ms?: number }
  | { event_name: 'profile.confirmation.viewed'; user_id?: string; completion_rate: number; missing_dimensions?: string[] }
  | { event_name: 'profile.fact.confirmed'; user_id?: string; fact_type: string; fact_value: string }
  | { event_name: 'profile.fact.dismissed'; fact_type: string; fact_value: string }
  | { event_name: 'find.intent.submitted'; user_id?: string; query: string; query_length: number }
  | { event_name: 'compliance.export.completed' }
  | { event_name: 'compliance.correction.submitted' }
  | { event_name: 'compliance.deletion.submitted' }
  | { event_name: 'safety.report.submitted' }
  | { event_name: 'safety.block.submitted' }
  | { event_name: 'recommendation.exposed'; user_id?: string; result_set_id: string; candidate_count: number; position: number }
  | { event_name: 'recommendation.detail.viewed'; recommendation_id: string }
  | { event_name: 'recommendation.feedback.submitted'; recommendation_id: string; feedback_type: string }
  | { event_name: 'dm.first_message.sent'; recommendation_id: string }
  | { event_name: 'settings.saved' }
  | { event_name: 'app_boot_started' }
  | { event_name: 'app_boot_finished' }
  | { event_name: 'app_boot_failed' }
  | { event_name: 'login_submit'; provider: string }
  | { event_name: 'login_success'; user_id: string; provider: string }
  | { event_name: 'login_failed'; error_message?: string }
  | { event_name: 'register_submit'; provider: string }
  | { event_name: 'register_success'; user_id: string; provider: string }
  | { event_name: 'register_failed'; error_message?: string }
  | { event_name: 'conversation_list_view' }
  | { event_name: 'conversation_open'; conversation_id: string }
  | { event_name: 'conversation_create'; conversation_id: string }
  | { event_name: 'chat_view' }
  | { event_name: 'chat_send'; conversation_id: string; content_type: 'text' | 'image' | 'voice' }
  | { event_name: 'chat_reply_received'; conversation_id: string; response_latency_ms?: number }
  | { event_name: 'chat_retry'; conversation_id: string }
  | { event_name: 'profile_fact_exposed'; completion_rate: number; missing_dimensions: string[] }
  | { event_name: 'profile_fact_accept'; fact_type: string; fact_value: string }
  | { event_name: 'profile_fact_reject'; fact_type: string; fact_value: string }
  | { event_name: 'profile_fact_snooze'; fact_type: string; fact_value: string }
  | { event_name: 'settings_view' }
  | { event_name: 'settings_section_open' }
  | { event_name: 'find_request_started' }
  | { event_name: 'find_request_submitted'; query: string; query_length: number }
  | { event_name: 'find_request_failed'; error_message?: string }
  | { event_name: 'clarification_view' }
  | { event_name: 'clarification_submit'; query: string; query_length: number }
  | { event_name: 'recommendation_list_view' }
  | { event_name: 'recommendation_card_exposed'; result_set_id: string; candidate_count: number; position: number }
  | { event_name: 'recommendation_detail_view'; recommendation_id: string }
  | { event_name: 'recommendation_explanation_view'; recommendation_id: string }
  | { event_name: 'recommendation_connect_start'; recommendation_id: string }
  | { event_name: 'recommendation_feedback_open'; recommendation_id: string }
  | { event_name: 'recommendation_feedback_submit'; recommendation_id: string; feedback_type: string }
  | { event_name: 'dm_first_message_submit'; recommendation_id: string }
  | { event_name: 'dm_first_message_approved'; thread_id: string; recipient_user_id: string }
  | { event_name: 'dm_first_message_blocked' }
  | { event_name: 'report_open' }
  | { event_name: 'report_submit'; target_type: 'user' | 'message' | 'profile'; target_id: string; reason: string }
  | { event_name: 'locale_setting_view' }
  | { event_name: 'locale_setting_save' }
  | { event_name: 'data_rights_view' }
  | { event_name: 'data_export_request' }
  | { event_name: 'data_delete_request' }
  | { event_name: 'data_correction_request' }
  | { event_name: 'error.occurred'; error_type: 'network' | 'auth' | 'validation' | 'runtime' | 'unknown'; error_code?: string; error_message?: string; http_status?: number };

export interface AnalyticsContext {
  user_id: string | null;
  session_id: string;
  platform: 'ios' | 'android' | 'web';
  app_version: string;
  screen: string;
  trace_id?: string;
}

export function createAnalyticsEvent(
  event: AnalyticsEvent,
  context: AnalyticsContext,
): Record<string, unknown> {
  return {
    ...event,
    occurred_at: new Date().toISOString(),
    session_id: context.session_id,
    platform: context.platform,
    app_version: context.app_version,
    screen: context.screen,
    user_id: context.user_id,
    trace_id: context.trace_id,
  };
}

let analyticsEndpoint = '/api/v1/bff/analytics/events';

export function setAnalyticsEndpoint(url: string) {
  analyticsEndpoint = url;
}

export async function trackEvent(event: AnalyticsEvent, context: AnalyticsContext): Promise<void> {
  const payload = createAnalyticsEvent(event, context);
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  const token = localStorage.getItem('onelink_token');
  if (token) { headers['Authorization'] = `Bearer ${token}`; }
  try {
    await fetch(analyticsEndpoint, {
      method: 'POST',
      headers,
      body: JSON.stringify(payload),
      keepalive: true,
    });
  } catch {
    // Analytics must not break user flows; swallow transport errors
  }
}

export function getAnalyticsContext(overrides?: Partial<AnalyticsContext>): AnalyticsContext {
  // JWT decode is for analytics context only — NOT used for auth or access control.
  // Server-side auth is enforced by BFF/identity-service via Authorization header.
  // The BFF analytics endpoint overrides user_id from the validated token,
  // so the client-supplied user_id here is only a fallback hint, not trusted.
  const token = localStorage.getItem('onelink_token');
  let userId: string | null = null;
  if (token) {
    try {
      const payload = JSON.parse(atob(token.split('.')[0] || ''));
      userId = payload?.user_id ?? null;
    } catch {
      // ignore
    }
  }
  return {
    user_id: userId,
    session_id: sessionStorage.getItem('onelink_session_id') || crypto.randomUUID(),
    platform: 'web',
    app_version: '1.0.0',
    screen: window.location.pathname,
    ...overrides,
  };
}
