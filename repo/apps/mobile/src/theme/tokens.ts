export const colors = {
  primary: '#4F46E5',
  primaryHover: '#4338CA',
  primaryLight: '#EEF2FF',
  secondary: '#7C3AED',
  background: '#FFFFFF',
  backgroundSecondary: '#F9FAFB',
  surface: '#F3F4F6',
  border: '#E5E7EB',
  borderFocus: '#6366F1',
  textPrimary: '#111827',
  textSecondary: '#6B7280',
  textPlaceholder: '#9CA3AF',
  success: '#059669',
  successBg: '#ECFDF5',
  warning: '#D97706',
  warningBg: '#FFFBEB',
  error: '#DC2626',
  errorBg: '#FEF2F2',
  info: '#2563EB',
  infoBg: '#EFF6FF',
  chatUserBubble: '#4F46E5',
  chatUserBubbleText: '#FFFFFF',
  chatAiBubble: '#F3F4F6',
  chatAiBubbleText: '#111827',
} as const;

export const spacing = {
  xs: 4,
  sm: 8,
  md: 12,
  lg: 16,
  xl: 24,
  xxl: 32,
  xxxl: 48,
} as const;

export const borderRadius = {
  sm: 6,
  md: 8,
  lg: 12,
  xl: 16,
  full: 9999,
} as const;

export const typography = {
  fontFamily: {
    base: "'Inter', 'Noto Sans SC', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    mono: "'JetBrains Mono', 'Fira Code', monospace",
  },
  fontSize: {
    xs: 12,
    sm: 14,
    base: 16,
    lg: 18,
    xl: 20,
    xxl: 24,
    xxxl: 30,
  },
  fontWeight: {
    normal: '400' as const,
    medium: '500' as const,
    semibold: '600' as const,
    bold: '700' as const,
  },
  lineHeight: {
    tight: 1.25,
    normal: 1.5,
    relaxed: 1.75,
  },
} as const;

export const componentStates = {
  empty: { colorToken: 'textSecondary' },
  loading: { colorToken: 'textSecondary' },
  error: { colorToken: 'error' },
  success: { colorToken: 'success' },
  degraded: { colorToken: 'warning' },
} as const;
