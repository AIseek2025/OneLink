import React from 'react';
import { tokens } from '../design-tokens';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
type Size = 'sm' | 'md' | 'lg';

interface OlButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  loading?: boolean;
  fullWidth?: boolean;
}

const variantStyles: Record<Variant, React.CSSProperties> = {
  primary: {
    background: tokens.color.brand.primary,
    color: '#FFFFFF',
    border: 'none',
  },
  secondary: {
    background: tokens.color.neutral.surface,
    color: tokens.color.neutral['text-primary'],
    border: `1px solid ${tokens.color.neutral.border}`,
  },
  ghost: {
    background: 'transparent',
    color: tokens.color.brand.primary,
    border: 'none',
  },
  danger: {
    background: tokens.color.semantic.error,
    color: '#FFFFFF',
    border: 'none',
  },
};

const sizeStyles: Record<Size, React.CSSProperties> = {
  sm: {
    padding: `${tokens.spacing.sm} ${tokens.spacing.md}`,
    fontSize: tokens.typography.fontSize.sm,
  },
  md: {
    padding: `${tokens.spacing.md} ${tokens.spacing.lg}`,
    fontSize: tokens.typography.fontSize.base,
  },
  lg: {
    padding: `${tokens.spacing.md} ${tokens.spacing.xl}`,
    fontSize: tokens.typography.fontSize.lg,
  },
};

export function OlButton({
  variant = 'primary',
  size = 'md',
  loading = false,
  fullWidth = false,
  disabled,
  children,
  style,
  ...rest
}: OlButtonProps) {
  const isDisabled = disabled || loading;
  return (
    <button
      disabled={isDisabled}
      style={{
        ...variantStyles[variant],
        ...sizeStyles[size],
        borderRadius: tokens.borderRadius.md,
        fontWeight: tokens.typography.fontWeight.semibold,
        cursor: isDisabled ? 'not-allowed' : 'pointer',
        opacity: isDisabled ? 0.5 : 1,
        width: fullWidth ? '100%' : undefined,
        transition: `background var(--ol-motion-fast, 150ms) ease-in-out, opacity var(--ol-motion-fast, 150ms) ease-in-out`,
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: tokens.spacing.sm,
        lineHeight: tokens.typography.lineHeight.normal,
        ...style,
      }}
      {...rest}
    >
      {loading && <OlSpinner size={16} />}
      {children}
    </button>
  );
}

export function OlSpinner({ size = 20, color }: { size?: number; color?: string }) {
  return (
    <span
      style={{
        display: 'inline-block',
        width: size,
        height: size,
        border: `2px solid ${color ?? tokens.color.neutral.border}`,
        borderTopColor: color ?? tokens.color.brand.primary,
        borderRadius: tokens.borderRadius.full,
        animation: 'ol-spin 0.6s linear infinite',
      }}
    />
  );
}

const keyframes = `@keyframes ol-spin { to { transform: rotate(360deg); } }`;

export function OlSpinnerStyle() {
  return <style>{keyframes}</style>;
}
