import { trackEvent, getAnalyticsContext, AnalyticsEvent, AnalyticsContext } from './analytics';

export type ScreenSpecEventName =
  | 'page_view'
  | 'registration_started'
  | 'registration_completed'
  | 'login_started'
  | 'login_completed'
  | 'chat_message_sent'
  | 'chat_message_received'
  | 'profile_confirmation_viewed'
  | 'profile_fact_confirmed'
  | 'profile_fact_dismissed'
  | 'find_intent_submitted'
  | 'app_boot_started'
  | 'app_boot_finished'
  | 'app_boot_failed'
  | 'login_submit'
  | 'login_success'
  | 'login_failed'
  | 'register_submit'
  | 'register_success'
  | 'register_failed'
  | 'conversation_list_view'
  | 'conversation_open'
  | 'conversation_create'
  | 'chat_view'
  | 'chat_send'
  | 'chat_reply_received'
  | 'chat_retry'
  | 'profile_fact_exposed'
  | 'profile_fact_accept'
  | 'profile_fact_reject'
  | 'profile_fact_snooze'
  | 'find_request_started'
  | 'find_request_submitted'
  | 'find_request_failed'
  | 'clarification_view'
  | 'clarification_submit'
  | 'recommendation_list_view'
  | 'recommendation_card_exposed'
  | 'recommendation_detail_view'
  | 'recommendation_explanation_view'
  | 'recommendation_connect_start'
  | 'recommendation_feedback_open'
  | 'recommendation_feedback_submit'
  | 'dm_first_message_submit'
  | 'dm_first_message_approved'
  | 'dm_first_message_blocked'
  | 'report_open'
  | 'report_submit'
  | 'settings_view'
  | 'settings_section_open'
  | 'locale_setting_view'
  | 'locale_setting_save'
  | 'data_rights_view'
  | 'data_export_request'
  | 'data_delete_request'
  | 'data_correction_request';

const SCREEN_SPEC_TO_ANALYTICS: Record<ScreenSpecEventName, (params?: Record<string, unknown>) => AnalyticsEvent> = {
  page_view: (p) => ({ event_name: 'page.view', page_path: (p?.page_path as string) || '/', referrer: (p?.referrer as string) || undefined }),
  registration_started: (p) => ({ event_name: 'registration.started', provider: (p?.provider as string) || 'email' }),
  registration_completed: (p) => ({ event_name: 'registration.completed', user_id: (p?.user_id as string) || '', provider: (p?.provider as string) || 'email' }),
  login_started: (p) => ({ event_name: 'login.started', provider: (p?.provider as string) || 'email' }),
  login_completed: (p) => ({ event_name: 'login.completed', user_id: (p?.user_id as string) || '', provider: (p?.provider as string) || 'email' }),
  chat_message_sent: (p) => ({ event_name: 'chat.message.sent', conversation_id: (p?.conversation_id as string) || '', content_type: (p?.content_type as 'text' | 'image' | 'voice') || 'text' }),
  chat_message_received: (p) => ({ event_name: 'chat.message.received', conversation_id: (p?.conversation_id as string) || '' }),
  profile_confirmation_viewed: (p) => ({ event_name: 'profile.confirmation.viewed', completion_rate: (p?.completion_rate as number) || 0 }),
  profile_fact_confirmed: (p) => ({ event_name: 'profile.fact.confirmed', fact_type: (p?.fact_type as string) || '', fact_value: (p?.fact_value as string) || '' }),
  profile_fact_dismissed: (p) => ({ event_name: 'profile.fact.dismissed', fact_type: (p?.fact_type as string) || '', fact_value: (p?.fact_value as string) || '' }),
  find_intent_submitted: (p) => ({ event_name: 'find.intent.submitted', query: (p?.query as string) || '', query_length: (p?.query_length as number) || 0 }),
  app_boot_started: () => ({ event_name: 'app_boot_started' }),
  app_boot_finished: () => ({ event_name: 'app_boot_finished' }),
  app_boot_failed: () => ({ event_name: 'app_boot_failed' }),
  login_submit: (p) => ({ event_name: 'login_submit', provider: (p?.provider as string) || 'email' }),
  login_success: (p) => ({ event_name: 'login_success', user_id: (p?.user_id as string) || '', provider: (p?.provider as string) || 'email' }),
  login_failed: (p) => ({ event_name: 'login_failed', error_message: (p?.error_message as string) || 'login_failed' }),
  register_submit: (p) => ({ event_name: 'register_submit', provider: (p?.provider as string) || 'email' }),
  register_success: (p) => ({ event_name: 'register_success', user_id: (p?.user_id as string) || '', provider: (p?.provider as string) || 'email' }),
  register_failed: (p) => ({ event_name: 'register_failed', error_message: (p?.error_message as string) || 'register_failed' }),
  conversation_list_view: () => ({ event_name: 'conversation_list_view' }),
  conversation_open: (p) => ({ event_name: 'conversation_open', conversation_id: (p?.conversation_id as string) || '' }),
  conversation_create: (p) => ({ event_name: 'conversation_create', conversation_id: (p?.conversation_id as string) || '' }),
  chat_view: () => ({ event_name: 'chat_view' }),
  chat_send: (p) => ({ event_name: 'chat_send', conversation_id: (p?.conversation_id as string) || '', content_type: (p?.content_type as 'text' | 'image' | 'voice') || 'text' }),
  chat_reply_received: (p) => ({ event_name: 'chat_reply_received', conversation_id: (p?.conversation_id as string) || '' }),
  chat_retry: (p) => ({ event_name: 'chat_retry', conversation_id: (p?.conversation_id as string) || '' }),
  profile_fact_exposed: (p) => ({ event_name: 'profile_fact_exposed', completion_rate: (p?.completion_rate as number) || 0, missing_dimensions: (p?.missing_dimensions as string[]) || [] }),
  profile_fact_accept: (p) => ({ event_name: 'profile_fact_accept', fact_type: (p?.fact_type as string) || '', fact_value: (p?.fact_value as string) || '' }),
  profile_fact_reject: (p) => ({ event_name: 'profile_fact_reject', fact_type: (p?.fact_type as string) || '', fact_value: (p?.fact_value as string) || '' }),
  profile_fact_snooze: (p) => ({ event_name: 'profile_fact_snooze', fact_type: (p?.fact_type as string) || '', fact_value: (p?.fact_value as string) || '' }),
  find_request_started: () => ({ event_name: 'find_request_started' }),
  find_request_submitted: (p) => ({ event_name: 'find_request_submitted', query: (p?.query as string) || '', query_length: (p?.query_length as number) || 0 }),
  find_request_failed: (p) => ({ event_name: 'find_request_failed', error_message: (p?.error_message as string) || 'find_request_failed' }),
  clarification_view: () => ({ event_name: 'clarification_view' }),
  clarification_submit: (p) => ({ event_name: 'clarification_submit', query: (p?.query as string) || '', query_length: (p?.query_length as number) || 0 }),
  recommendation_list_view: () => ({ event_name: 'recommendation_list_view' }),
  recommendation_card_exposed: (p) => ({ event_name: 'recommendation_card_exposed', result_set_id: (p?.result_set_id as string) || '', candidate_count: (p?.candidate_count as number) || 0, position: (p?.position as number) || 0 }),
  recommendation_detail_view: (p) => ({ event_name: 'recommendation_detail_view', recommendation_id: (p?.recommendation_id as string) || '' }),
  recommendation_explanation_view: (p) => ({ event_name: 'recommendation_explanation_view', recommendation_id: (p?.recommendation_id as string) || '' }),
  recommendation_connect_start: (p) => ({ event_name: 'recommendation_connect_start', recommendation_id: (p?.recommendation_id as string) || '' }),
  recommendation_feedback_open: (p) => ({ event_name: 'recommendation_feedback_open', recommendation_id: (p?.recommendation_id as string) || '' }),
  recommendation_feedback_submit: (p) => ({ event_name: 'recommendation_feedback_submit', recommendation_id: (p?.recommendation_id as string) || '', feedback_type: (p?.feedback_type as string) || 'neutral' }),
  dm_first_message_submit: (p) => ({ event_name: 'dm_first_message_submit', recommendation_id: (p?.recommendation_id as string) || '' }),
  dm_first_message_approved: (p) => ({ event_name: 'dm_first_message_approved', thread_id: (p?.thread_id as string) || '', recipient_user_id: (p?.recipient_user_id as string) || '' }),
  dm_first_message_blocked: () => ({ event_name: 'dm_first_message_blocked' }),
  report_open: () => ({ event_name: 'report_open' }),
  report_submit: (p) => ({ event_name: 'report_submit', target_type: (p?.target_type as 'user' | 'message' | 'profile') || 'user', target_id: (p?.target_id as string) || '', reason: (p?.reason as string) || '' }),
  settings_view: () => ({ event_name: 'settings_view' }),
  settings_section_open: () => ({ event_name: 'settings_section_open' }),
  locale_setting_view: () => ({ event_name: 'locale_setting_view' }),
  locale_setting_save: () => ({ event_name: 'locale_setting_save' }),
  data_rights_view: () => ({ event_name: 'data_rights_view' }),
  data_export_request: () => ({ event_name: 'data_export_request' }),
  data_delete_request: () => ({ event_name: 'data_delete_request' }),
  data_correction_request: () => ({ event_name: 'data_correction_request' }),
};

export function trackScreenSpecEvent(
  eventName: ScreenSpecEventName,
  params?: Record<string, unknown>,
  contextOverrides?: Partial<AnalyticsContext>,
): void {
  const mapper = SCREEN_SPEC_TO_ANALYTICS[eventName];
  if (!mapper) return;
  const analyticsEvent = mapper(params);
  getAnalyticsContext(contextOverrides).then((ctx) => {
    trackEvent(analyticsEvent, ctx);
  });
}

export type { AnalyticsEvent, AnalyticsContext };
export { trackEvent, getAnalyticsContext };
