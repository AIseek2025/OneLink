export const tokens = {
  color: {
    brand: {
      primary: '#4F46E5',
      'primary-hover': '#4338CA',
      'primary-light': '#EEF2FF',
      secondary: '#7C3AED',
    },
    neutral: {
      bg: '#FFFFFF',
      'bg-secondary': '#F9FAFB',
      surface: '#F3F4F6',
      border: '#E5E7EB',
      'border-focus': '#6366F1',
      'text-primary': '#111827',
      'text-secondary': '#6B7280',
      'text-placeholder': '#9CA3AF',
    },
    semantic: {
      success: '#059669',
      'success-bg': '#ECFDF5',
      warning: '#D97706',
      'warning-bg': '#FFFBEB',
      error: '#DC2626',
      'error-bg': '#FEF2F2',
      info: '#2563EB',
      'info-bg': '#EFF6FF',
    },
    chat: {
      'user-bubble': '#4F46E5',
      'user-bubble-text': '#FFFFFF',
      'ai-bubble': '#F3F4F6',
      'ai-bubble-text': '#111827',
    },
  },
  typography: {
    fontFamily: {
      base: "'Inter', 'Noto Sans SC', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      mono: "'JetBrains Mono', 'Fira Code', monospace",
    },
    fontSize: {
      xs: '12px',
      sm: '14px',
      base: '16px',
      lg: '18px',
      xl: '20px',
      '2xl': '24px',
      '3xl': '30px',
    },
    fontWeight: {
      normal: 400,
      medium: 500,
      semibold: 600,
      bold: 700,
    },
    lineHeight: {
      tight: 1.25,
      normal: 1.5,
      relaxed: 1.75,
    },
  },
  spacing: {
    xs: '4px',
    sm: '8px',
    md: '12px',
    lg: '16px',
    xl: '24px',
    '2xl': '32px',
    '3xl': '48px',
  },
  borderRadius: {
    sm: '6px',
    md: '8px',
    lg: '12px',
    xl: '16px',
    full: '9999px',
  },
  shadow: {
    sm: '0 1px 2px 0 rgba(0,0,0,0.05)',
    md: '0 4px 6px -1px rgba(0,0,0,0.1), 0 2px 4px -2px rgba(0,0,0,0.1)',
    lg: '0 10px 15px -3px rgba(0,0,0,0.1), 0 4px 6px -4px rgba(0,0,0,0.1)',
  },
  breakpoint: {
    sm: '640px',
    md: '768px',
    lg: '1024px',
    xl: '1280px',
  },
  zIndex: {
    dropdown: 1000,
    sticky: 1100,
    'modal-backdrop': 1200,
    modal: 1300,
    toast: 1400,
  },
} as const;

export type DesignTokens = typeof tokens;
