export interface User {
  user_id: string;
  nickname: string;
  avatar_url?: string;
  first_run: boolean;
  locale: string;
  region: string;
}

export interface BootResponse {
  boot_state: {
    state: string;
    has_session: boolean;
    user: User | null;
  };
}

export interface AuthResponse {
  user_id: string;
  access_token: string;
  refresh_token: string;
  flow_state: string;
}

export interface ConversationSummary {
  conversation_id: string;
  title: string;
  last_message_preview?: string;
  updated_at: string;
}

export interface FindRequestCreate {
  intent_text: string;
  preferred_region?: string;
  preferred_locale?: string;
}

export interface Recommendation {
  recommendation_id: string;
  user_id: string;
  nickname: string;
  avatar_url?: string;
  reason: string;
  feedback_state: string;
}

export interface DmDraftRequest {
  recommendation_id: string;
  initial_message: string;
}

export interface SettingsDto {
  notifications_enabled: boolean;
  language: string;
  theme: string;
  locale: string;
  region: string;
  timezone: string;
  content_language: string;
  notification_language: string;
}

export interface ComplianceActionRequest {
  action_type: 'export' | 'delete' | 'correction';
  scope?: string;
  field_name?: string;
  export_format?: string;
}
