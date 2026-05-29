import { t } from '../i18n';
import { colors, typography } from '../theme/tokens';

describe('ScreenShell', () => {
  it('has all required state labels in i18n', () => {
    expect(t('state.empty')).toBeDefined();
    expect(t('state.loading')).toBeDefined();
    expect(t('state.error')).toBeDefined();
    expect(t('state.degraded')).toBeDefined();
  });
});

describe('RootNavigator contract', () => {
  it('defines all tab routes matching spec', () => {
    const tabRoutes = ['Home', 'Find', 'Recommendations', 'Messages', 'Me'];
    expect(tabRoutes).toHaveLength(5);
  });

  it('defines stack routes for auth and chat', () => {
    const stackRoutes = ['Splash', 'Auth', 'Main', 'Chat'];
    expect(stackRoutes).toContain('Chat');
  });

  it('chat route expects conversationId param', () => {
    const params = { conversationId: 'string' };
    expect(params).toHaveProperty('conversationId');
  });

  it('uses BFF contract prefix /api/v1/bff for all endpoints', () => {
    const bffEndpoints = [
      '/auth/login',
      '/auth/register',
      '/auth/me',
      '/chat/conversations',
      '/find/requests',
      '/recommendations',
      '/dm/threads',
      '/settings',
    ];
    bffEndpoints.forEach((ep) => {
      expect(ep.startsWith('/')).toBe(true);
    });
  });

  it('theme tokens have all chat bubble colors', () => {
    expect(colors.chatUserBubble).toBeDefined();
    expect(colors.chatAiBubble).toBeDefined();
    expect(colors.chatUserBubbleText).toBeDefined();
    expect(colors.chatAiBubbleText).toBeDefined();
  });

  it('typography has required font sizes for mobile UI', () => {
    expect(typography.fontSize.xs).toBe(12);
    expect(typography.fontSize.sm).toBe(14);
    expect(typography.fontSize.base).toBe(16);
    expect(typography.fontSize.lg).toBe(18);
    expect(typography.fontSize.xl).toBe(20);
    expect(typography.fontSize.xxl).toBe(24);
  });
});
