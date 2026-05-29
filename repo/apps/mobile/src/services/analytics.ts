import { Platform } from 'react-native';
import { bffUrl } from './config';
import { getAuthToken } from './bffClient';
import AsyncStorage from '@react-native-async-storage/async-storage';

export type AnalyticsEvent =
  | { event_name: 'page.view'; page_path: string; referrer?: string }
  | { event_name: 'registration.started'; provider?: string }
  | { event_name: 'registration.completed'; user_id: string; provider: string }
  | { event_name: 'login.started'; provider?: string }
  | { event_name: 'login.completed'; user_id: string; provider: string }
  | { event_name: 'chat.message.sent'; conversation_id: string; content_type: 'text' | 'image' | 'voice' }
  | { event_name: 'chat.message.received'; conversation_id: string; response_latency_ms?: number }
  | { event_name: 'profile.confirmation.viewed'; completion_rate: number }
  | { event_name: 'profile.fact.confirmed'; fact_type: string; fact_value: string }
  | { event_name: 'profile.fact.dismissed'; fact_type: string; fact_value: string }
  | { event_name: 'find.intent.submitted'; query: string; query_length: number }
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
  | { event_name: 'settings.saved' }
  | { event_name: 'error.occurred'; error_type: 'network' | 'auth' | 'validation' | 'runtime' | 'unknown'; error_code?: string; error_message?: string; http_status?: number };

export interface AnalyticsContext {
  user_id: string | null;
  session_id: string;
  platform: 'ios' | 'android';
  app_version: string;
  screen: string;
  trace_id?: string;
}

const SESSION_ID_KEY = 'onelink_analytics_session_id';
const APP_VERSION = '0.3.0';

let analyticsEndpointOverride: string | null = null;

export function setAnalyticsEndpoint(url: string) {
  analyticsEndpointOverride = url;
}

function getAnalyticsEndpoint(): string {
  return analyticsEndpointOverride ?? bffUrl('/analytics/events');
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

export async function trackEvent(event: AnalyticsEvent, context: AnalyticsContext): Promise<void> {
  const payload = createAnalyticsEvent(event, context);
  enqueueEvent(payload);
}

interface QueuedEvent {
  payload: Record<string, unknown>;
  timestamp: number;
}

const MAX_BATCH_SIZE = 20;
const FLUSH_INTERVAL_MS = 5000;
let queue: QueuedEvent[] = [];
let flushTimer: ReturnType<typeof setInterval> | null = null;

function enqueueEvent(payload: Record<string, unknown>): void {
  queue.push({ payload, timestamp: Date.now() });
  if (queue.length >= MAX_BATCH_SIZE) {
    void flushQueue();
  } else if (!flushTimer) {
    flushTimer = setInterval(() => void flushQueue(), FLUSH_INTERVAL_MS);
  }
}

export async function flushQueue(): Promise<void> {
  if (flushTimer) {
    clearInterval(flushTimer);
    flushTimer = null;
  }
  if (queue.length === 0) return;
  const batch = queue.splice(0, MAX_BATCH_SIZE);
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  const token = getAuthToken();
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  try {
    await fetch(getAnalyticsEndpoint(), {
      method: 'POST',
      headers,
      body: JSON.stringify({ events: batch.map((e) => e.payload) }),
    });
  } catch {
    // Analytics must not break user flows; swallow transport errors
  }
}

export function getQueueLength(): number {
  return queue.length;
}

export function resetQueue(): void {
  queue = [];
  if (flushTimer) {
    clearInterval(flushTimer);
    flushTimer = null;
  }
}

function createSessionId(): string {
  return `session-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

export async function getAnalyticsContext(overrides?: Partial<AnalyticsContext>): Promise<AnalyticsContext> {
  let sessionId = await AsyncStorage.getItem(SESSION_ID_KEY);
  if (!sessionId) {
    sessionId = globalThis.crypto?.randomUUID?.() ?? createSessionId();
    await AsyncStorage.setItem(SESSION_ID_KEY, sessionId);
  }
  const platform = Platform.OS === 'ios' ? 'ios' : 'android';
  return {
    user_id: null,
    session_id: sessionId,
    platform,
    app_version: APP_VERSION,
    screen: '/',
    ...overrides,
  };
}
