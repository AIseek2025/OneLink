import { User, AuthResponse, ConversationSummary, FindRequestCreate, Recommendation, SettingsDto, ComplianceActionRequest, DmDraftRequest, BootResponse } from '../types';

describe('types/index.ts', () => {
  it('User has required fields', () => {
    const user: User = {
      user_id: 'u1',
      nickname: 'Test',
      first_run: false,
      locale: 'zh-CN',
      region: 'CN',
    };
    expect(user.user_id).toBe('u1');
    expect(user.nickname).toBe('Test');
  });

  it('AuthResponse has required fields', () => {
    const auth: AuthResponse = {
      user_id: 'u1',
      access_token: 'at',
      refresh_token: 'rt',
      flow_state: 'authenticated',
    };
    expect(auth.access_token).toBe('at');
    expect(auth.flow_state).toBe('authenticated');
  });

  it('ConversationSummary has required fields', () => {
    const conv: ConversationSummary = {
      conversation_id: 'c1',
      title: 'Chat',
      updated_at: '2026-01-01',
    };
    expect(conv.conversation_id).toBe('c1');
  });

  it('FindRequestCreate has intent_text', () => {
    const req: FindRequestCreate = { intent_text: '找朋友' };
    expect(req.intent_text).toBe('找朋友');
  });

  it('Recommendation has required fields', () => {
    const rec: Recommendation = {
      recommendation_id: 'r1',
      user_id: 'u2',
      nickname: 'Rec User',
      reason: 'Matched',
      feedback_state: 'none',
    };
    expect(rec.recommendation_id).toBe('r1');
  });

  it('SettingsDto has required fields', () => {
    const settings: SettingsDto = {
      notifications_enabled: true,
      language: 'zh',
      theme: 'light',
      locale: 'zh-CN',
      region: 'CN',
      timezone: 'Asia/Shanghai',
      content_language: 'zh',
      notification_language: 'zh',
    };
    expect(settings.notifications_enabled).toBe(true);
  });

  it('ComplianceActionRequest supports action types', () => {
    const exportReq: ComplianceActionRequest = { action_type: 'export' };
    const deleteReq: ComplianceActionRequest = { action_type: 'delete' };
    const correctionReq: ComplianceActionRequest = { action_type: 'correction' };
    expect(exportReq.action_type).toBe('export');
    expect(deleteReq.action_type).toBe('delete');
    expect(correctionReq.action_type).toBe('correction');
  });

  it('BootResponse has boot_state with has_session', () => {
    const boot: BootResponse = {
      boot_state: { state: 'ready', has_session: true, user: null },
    };
    expect(boot.boot_state.has_session).toBe(true);
  });

  it('DmDraftRequest has recommendation_id and initial_message', () => {
    const dm: DmDraftRequest = { recommendation_id: 'r1', initial_message: 'Hi' };
    expect(dm.recommendation_id).toBe('r1');
    expect(dm.initial_message).toBe('Hi');
  });
});
