const BFF_BASE = '/api/v1/bff';

function getAuthHeader(): Record<string, string> {
  const token = localStorage.getItem('onelink_token');
  return token ? { Authorization: `Bearer ${token}` } : {};
}

export async function register(email: string, password: string, primary_region: string, primary_language: string) {
  const res = await fetch(`${BFF_BASE}/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ provider: 'email', email, password, primary_region, primary_language }),
  });
  if (!res.ok) throw new Error(`register failed: ${res.status}`);
  return res.json();
}

export async function login(email: string, password: string) {
  const res = await fetch(`${BFF_BASE}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ provider: 'email', email, password }),
  });
  if (!res.ok) throw new Error(`login failed: ${res.status}`);
  const data = await res.json();
  localStorage.setItem('onelink_token', data.session.token);
  return data;
}

export async function fetchHome() {
  const res = await fetch(`${BFF_BASE}/home`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`home failed: ${res.status}`);
  return res.json();
}

export async function fetchChatInit() {
  const res = await fetch(`${BFF_BASE}/chat/init`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`chat init failed: ${res.status}`);
  return res.json();
}

export async function fetchProfile(userId: string) {
  const res = await fetch(`${BFF_BASE}/profile/${userId}`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`profile failed: ${res.status}`);
  return res.json();
}

export async function submitFindIntent(rawQuery: string, intentTags?: string[]) {
  const res = await fetch(`${BFF_BASE}/find/intent`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ raw_query: rawQuery, intent_tags: intentTags ?? [] }),
  });
  if (!res.ok) throw new Error(`find intent failed: ${res.status}`);
  return res.json();
}

export async function sendMessage(conversationId: string, contentText: string) {
  const res = await fetch(`${BFF_BASE}/chat/messages`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ conversation_id: conversationId, content_type: 'text', content_text: contentText }),
  });
  if (!res.ok) throw new Error(`send message failed: ${res.status}`);
  return res.json();
}

export async function fetchOnboarding() {
  const res = await fetch(`${BFF_BASE}/onboarding`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`onboarding failed: ${res.status}`);
  return res.json();
}

export async function answerQuestion(deliveryId: string, variantId: string | undefined, answer: string | string[]) {
  const res = await fetch(`${BFF_BASE}/questions/answers`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ delivery_id: deliveryId, variant_id: variantId, answer }),
  });
  if (!res.ok) throw new Error(`answer question failed: ${res.status}`);
  return res.json();
}

export async function patchProfile(fields: Record<string, unknown>) {
  const res = await fetch(`${BFF_BASE}/profile/me`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify(fields),
  });
  if (!res.ok) throw new Error(`patch profile failed: ${res.status}`);
  return res.json();
}

export async function fetchFindResults() {
  const res = await fetch(`${BFF_BASE}/find/results`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`find results failed: ${res.status}`);
  return res.json();
}

export async function fetchFindRequestDetail(requestId: string) {
  const res = await fetch(`${BFF_BASE}/find/requests/${requestId}`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`find detail failed: ${res.status}`);
  return res.json();
}

export async function submitClarification(requestId: string, answers: Record<string, string>) {
  const res = await fetch(`${BFF_BASE}/find/requests/${requestId}/clarifications`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ answers }),
  });
  if (!res.ok) throw new Error(`clarification failed: ${res.status}`);
  return res.json();
}

export async function fetchRecommendationList() {
  const res = await fetch(`${BFF_BASE}/recommendations`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`recommendations failed: ${res.status}`);
  return res.json();
}

export async function fetchRecommendationDetail(recId: string) {
  const res = await fetch(`${BFF_BASE}/recommendations/${recId}`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`recommendation detail failed: ${res.status}`);
  return res.json();
}

export async function submitRecommendationFeedback(recId: string, feedbackType: string, comment?: string) {
  const res = await fetch(`${BFF_BASE}/recommendations/${recId}/feedback`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ feedback_type: feedbackType, comment }),
  });
  if (!res.ok) throw new Error(`recommendation feedback failed: ${res.status}`);
  return res.json();
}

export async function createDmDraft(recommendationId: string, initialMessage: string) {
  const res = await fetch(`${BFF_BASE}/dm/threads/draft`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ recommendation_id: recommendationId, initial_message: initialMessage }),
  });
  if (!res.ok) throw new Error(`dm draft failed: ${res.status}`);
  return res.json();
}

export async function submitDmFirstMessage(threadId: string, message: string) {
  const res = await fetch(`${BFF_BASE}/dm/threads/first-message`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ thread_id: threadId, message }),
  });
  if (!res.ok) throw new Error(`dm first message failed: ${res.status}`);
  return res.json();
}

export async function fetchDmThreadDetail(threadId: string) {
  const res = await fetch(`${BFF_BASE}/dm/threads/${threadId}`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`dm thread detail failed: ${res.status}`);
  return res.json();
}

export async function fetchDmThreadList() {
  const res = await fetch(`${BFF_BASE}/conversations`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`dm thread list failed: ${res.status}`);
  return res.json();
}

export async function submitSafetyReport(targetUserId: string, category: string, description?: string) {
  const res = await fetch(`${BFF_BASE}/safety/reports`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ target_user_id: targetUserId, category, description }),
  });
  if (!res.ok) throw new Error(`safety report failed: ${res.status}`);
  return res.json();
}

export async function submitSafetyBlock(targetUserId: string, reason?: string) {
  const res = await fetch(`${BFF_BASE}/safety/blocks`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ target_user_id: targetUserId, reason }),
  });
  if (!res.ok) throw new Error(`safety block failed: ${res.status}`);
  return res.json();
}

export async function fetchAppealStatus(appealId: string) {
  const res = await fetch(`${BFF_BASE}/safety/appeals/${appealId}`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`appeal status failed: ${res.status}`);
  return res.json();
}

export async function fetchAdminReportQueue() {
  const res = await fetch(`${BFF_BASE}/admin/reports`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`admin reports failed: ${res.status}`);
  return res.json();
}

export async function fetchAdminReportDetail(reportId: string) {
  const res = await fetch(`${BFF_BASE}/admin/reports/${reportId}`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`admin report detail failed: ${res.status}`);
  return res.json();
}

export async function submitAdminReportAction(reportId: string, actionType: string, reason?: string) {
  const res = await fetch(`${BFF_BASE}/admin/reports/${reportId}/action`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ action_type: actionType, reason }),
  });
  if (!res.ok) throw new Error(`admin action failed: ${res.status}`);
  return res.json();
}

export async function fetchAdminAppealQueue() {
  const res = await fetch(`${BFF_BASE}/admin/appeals`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`admin appeals failed: ${res.status}`);
  return res.json();
}

export async function trackAnalyticsEvents(events: Record<string, unknown> | Record<string, unknown>[]) {
  const res = await fetch(`${BFF_BASE}/analytics/events`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify(events),
  });
  if (!res.ok) throw new Error(`analytics events failed: ${res.status}`);
  return res.json();
}

export async function fetchDmList() {
  const res = await fetch(`${BFF_BASE}/dm/list`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`dm list failed: ${res.status}`);
  return res.json();
}

export async function sendDm(recipientUserId: string, content: string) {
  const res = await fetch(`${BFF_BASE}/dm/send`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ recipient_user_id: recipientUserId, content }),
  });
  if (!res.ok) throw new Error(`dm send failed: ${res.status}`);
  return res.json();
}

export async function reportUser(reportedUserId: string, reason: string, description?: string) {
  const res = await fetch(`${BFF_BASE}/safety/report`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ reported_user_id: reportedUserId, reason, description }),
  });
  if (!res.ok) throw new Error(`safety report failed: ${res.status}`);
  return res.json();
}

export async function blockUser(blockedUserId: string) {
  const res = await fetch(`${BFF_BASE}/safety/block`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ blocked_user_id: blockedUserId }),
  });
  if (!res.ok) throw new Error(`safety block failed: ${res.status}`);
  return res.json();
}

export async function fetchComplianceSummary() {
  const res = await fetch(`${BFF_BASE}/compliance/summary`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`compliance summary failed: ${res.status}`);
  return res.json();
}

export async function requestComplianceExport(exportFormat?: string) {
  const res = await fetch(`${BFF_BASE}/compliance/export`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ action_type: 'export', export_format: exportFormat ?? 'json' }),
  });
  if (!res.ok) throw new Error(`compliance export failed: ${res.status}`);
  return res.json();
}

export async function requestComplianceCorrection(fieldName: string) {
  const res = await fetch(`${BFF_BASE}/compliance/correction`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ action_type: 'correction', field_name: fieldName }),
  });
  if (!res.ok) throw new Error(`compliance correction failed: ${res.status}`);
  return res.json();
}

export async function requestComplianceDelete(scope?: string) {
  const res = await fetch(`${BFF_BASE}/compliance/delete`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ action_type: 'delete', scope: scope ?? 'all' }),
  });
  if (!res.ok) throw new Error(`compliance delete failed: ${res.status}`);
  return res.json();
}

export async function fetchLocaleRegistry() {
  const res = await fetch(`${BFF_BASE}/settings/locale`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`locale registry failed: ${res.status}`);
  return res.json();
}

export async function fetchAdminMetrics() {
  const res = await fetch(`${BFF_BASE}/admin/metrics`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`admin metrics failed: ${res.status}`);
  return res.json();
}

export async function patchLocaleSettings(settings: { locale?: string; region?: string; timezone?: string; content_language?: string; notification_language?: string }) {
  const res = await fetch(`${BFF_BASE}/settings/locale`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify(settings),
  });
  if (!res.ok) throw new Error(`locale update failed: ${res.status}`);
  return res.json();
}

export async function patchUserSettings(fields: Record<string, unknown>) {
  const res = await fetch(`${BFF_BASE}/users/me`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify(fields),
  });
  if (!res.ok) throw new Error(`user settings update failed: ${res.status}`);
  return res.json();
}

export async function fetchUserMe() {
  const res = await fetch(`${BFF_BASE}/users/me`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`user me failed: ${res.status}`);
  return res.json();
}

export async function fetchComplianceData() {
  const res = await fetch(`${BFF_BASE}/compliance/data`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`compliance data failed: ${res.status}`);
  return res.json();
}

export async function requestComplianceDeletion(confirmation: string) {
  const res = await fetch(`${BFF_BASE}/compliance/deletion`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ confirmation }),
  });
  if (!res.ok) throw new Error(`compliance deletion failed: ${res.status}`);
  return res.json();
}

export async function requestComplianceCorrectionRaw(field: string, requestedValue: string) {
  const res = await fetch(`${BFF_BASE}/compliance/correction`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...getAuthHeader() },
    body: JSON.stringify({ field, requested_value: requestedValue }),
  });
  if (!res.ok) throw new Error(`compliance correction failed: ${res.status}`);
  return res.json();
}

export async function requestComplianceExportRaw() {
  const res = await fetch(`${BFF_BASE}/compliance/export`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`compliance export failed: ${res.status}`);
  return res;
}

export async function fetchRegionGate(userRegion?: string) {
  const headers: Record<string, string> = { ...getAuthHeader() };
  if (userRegion) headers['X-User-Region'] = userRegion;
  const res = await fetch(`${BFF_BASE}/region/gate`, { headers });
  if (!res.ok) throw new Error(`region gate failed: ${res.status}`);
  return res.json();
}

export async function fetchI18nRegistry() {
  const res = await fetch(`${BFF_BASE}/i18n/registry`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`i18n registry failed: ${res.status}`);
  return res.json();
}

export async function translateI18nKey(key: string, locale?: string) {
  const params = new URLSearchParams({ key });
  if (locale) params.set('locale', locale);
  const res = await fetch(`${BFF_BASE}/i18n/translate?${params.toString()}`, { headers: getAuthHeader() });
  if (!res.ok) throw new Error(`i18n translate failed: ${res.status}`);
  return res.json();
}