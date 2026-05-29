import React from 'react';
import { tokens } from '../design-tokens';

interface OlErrorBannerProps {
  message: string;
  onDismiss?: () => void;
  type?: 'error' | 'warning' | 'info';
}

const typeStyles: Record<string, { bg: string; color: string }> = {
  error: { bg: tokens.color.semantic['error-bg'], color: tokens.color.semantic.error },
  warning: { bg: tokens.color.semantic['warning-bg'], color: tokens.color.semantic.warning },
  info: { bg: tokens.color.semantic['info-bg'], color: tokens.color.semantic.info },
};

export function OlErrorBanner({ message, onDismiss, type = 'error' }: OlErrorBannerProps) {
  const styles = typeStyles[type];
  return (
    <div
      role="alert"
      style={{
        background: styles.bg,
        color: styles.color,
        padding: tokens.spacing.md,
        borderRadius: tokens.borderRadius.md,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        marginBottom: tokens.spacing.md,
      }}
    >
      <span style={{ fontSize: tokens.typography.fontSize.sm }}>{message}</span>
      {onDismiss && (
        <button
          onClick={onDismiss}
          style={{
            background: 'transparent',
            border: 'none',
            color: styles.color,
            cursor: 'pointer',
            fontSize: tokens.typography.fontSize.sm,
            padding: tokens.spacing.xs,
          }}
          aria-label="Dismiss"
        >
          ✕
        </button>
      )}
    </div>
  );
}

interface OlLoadingProps {
  text?: string;
}

export function OlLoading({ text = 'Loading...' }: OlLoadingProps) {
  return (
    <div
      style={{
        padding: tokens.spacing.xl,
        color: tokens.color.neutral['text-secondary'],
        textAlign: 'center',
      }}
    >
      {text}
    </div>
  );
}

interface OlPageShellProps {
  children: React.ReactNode;
  maxWidth?: number;
}

export function OlPageShell({ children, maxWidth = 640 }: OlPageShellProps) {
  return (
    <div
      style={{
        maxWidth,
        margin: '0 auto',
        padding: tokens.spacing.xl,
      }}
    >
      {children}
    </div>
  );
}