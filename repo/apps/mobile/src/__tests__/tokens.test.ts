import { colors, spacing, typography, borderRadius, componentStates } from '../theme/tokens';

describe('theme tokens', () => {
  it('exports colors matching web design tokens', () => {
    expect(colors.primary).toBe('#4F46E5');
    expect(colors.primaryHover).toBe('#4338CA');
    expect(colors.secondary).toBe('#7C3AED');
    expect(colors.background).toBe('#FFFFFF');
    expect(colors.error).toBe('#DC2626');
    expect(colors.success).toBe('#059669');
    expect(colors.chatUserBubble).toBe('#4F46E5');
    expect(colors.chatAiBubble).toBe('#F3F4F6');
  });

  it('exports spacing with all required keys', () => {
    expect(spacing.xs).toBe(4);
    expect(spacing.sm).toBe(8);
    expect(spacing.md).toBe(12);
    expect(spacing.lg).toBe(16);
    expect(spacing.xl).toBe(24);
    expect(spacing.xxl).toBe(32);
  });

  it('exports typography with font sizes and weights', () => {
    expect(typography.fontSize.base).toBe(16);
    expect(typography.fontSize.xxl).toBe(24);
    expect(typography.fontWeight.bold).toBe('700');
    expect(typography.fontWeight.semibold).toBe('600');
  });

  it('exports borderRadius', () => {
    expect(borderRadius.md).toBe(8);
    expect(borderRadius.lg).toBe(12);
    expect(borderRadius.full).toBe(9999);
  });

  it('exports componentStates', () => {
    expect(componentStates.empty).toBeDefined();
    expect(componentStates.error).toBeDefined();
    expect(componentStates.degraded).toBeDefined();
  });
});
