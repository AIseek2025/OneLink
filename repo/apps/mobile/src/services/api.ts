import { bffClient } from './bffClient';
import type {
  User,
  AuthResponse,
  ConversationSummary,
  FindRequestCreate,
  Recommendation,
  SettingsDto,
  ComplianceActionRequest,
} from '../types';

export async function register(email: string, password: string, nickname: string) {
  return bffClient.post<AuthResponse>('/auth/register', { email, password, nickname });
}

export async function login(email: string, password: string) {
  return bffClient.post<AuthResponse>('/auth/login', { email, password });
}

export async function fetchUserMe() {
  return bffClient.get<{ user: User }>('/users/me');
}

export async function patchProfile(fields: Record<string, unknown>) {
  return bffClient.patch<{ user: User }>('/users/me', fields);
}

export async function fetchConversations() {
  return bffClient.get<{ conversations: ConversationSummary[] }>('/chat/conversations');
}

export async function fetchMessages(conversationId: string) {
  return bffClient.get<{ messages: { message_id: string; role: string; content: string; created_at: string }[] }>(
    `/chat/conversations/${conversationId}/messages`,
  );
}

export async function sendMessage(conversationId: string, content: string) {
  return bffClient.post<{ message: { message_id: string; role: string; content: string; created_at: string } }>(
    `/chat/conversations/${conversationId}/messages`,
    { content },
  );
}

export async function submitFindIntent(payload: FindRequestCreate) {
  return bffClient.post<{ request_id: string; status: string }>('/find/requests', payload);
}

export async function fetchRecommendations() {
  return bffClient.get<{ recommendations: Recommendation[] }>('/recommendations');
}

export async function submitRecommendationFeedback(recId: string, feedbackType: string) {
  return bffClient.post(`/recommendations/${recId}/feedback`, { feedback_type: feedbackType });
}

export async function fetchDmThreads() {
  return bffClient.get<{ threads: { thread_id: string; recipient_nickname: string; last_message_preview?: string; updated_at: string }[] }>('/dm/threads');
}

export async function sendDmFirstMessage(threadId: string, content: string) {
  return bffClient.post('/dm/threads/${threadId}/messages', { content });
}

export async function fetchSettings() {
  return bffClient.get<{ settings: SettingsDto }>('/settings');
}

export async function patchLocaleSettings(fields: Record<string, string>) {
  return bffClient.patch('/settings/locale', fields);
}

export async function patchUserSettings(fields: Record<string, unknown>) {
  return bffClient.patch('/users/me', fields);
}

export async function fetchProfileFacts() {
  return bffClient.get<{ facts: { fact_key: string; fact_value: string; confidence: string; source: string; confirmed: boolean }[] }>('/users/me/facts');
}

export async function confirmProfileFact(factKey: string) {
  return bffClient.post(`/users/me/facts/${factKey}/confirm`, {});
}

export async function dismissProfileFact(factKey: string) {
  return bffClient.post(`/users/me/facts/${factKey}/dismiss`, {});
}

export async function submitSafetyReport(targetUserId: string, category: string, description?: string) {
  return bffClient.post('/safety/reports', { target_user_id: targetUserId, category, description });
}

export async function submitSafetyBlock(targetUserId: string, reason?: string) {
  return bffClient.post('/safety/blocks', { target_user_id: targetUserId, reason });
}

export async function fetchAppealStatus(appealId: string) {
  return bffClient.get<{ status: string }>(`/safety/appeals/${appealId}`);
}

export async function fetchComplianceData() {
  return bffClient.get<Record<string, unknown>>('/compliance/data');
}

export async function requestComplianceAction(payload: ComplianceActionRequest) {
  return bffClient.post('/compliance/actions', payload);
}

export async function trackAnalyticsEvents(events: Record<string, unknown>[]) {
  return bffClient.post('/analytics/events', { events });
}
