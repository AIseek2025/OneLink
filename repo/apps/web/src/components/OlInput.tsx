import type { CSSProperties, InputHTMLAttributes, TextareaHTMLAttributes } from 'react';
import { tokens } from '../design-tokens';

interface OlInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'size'> {
  label?: string;
  error?: string;
  size?: 'sm' | 'md' | 'lg';
}

const sizeStyles: Record<string, CSSProperties> = {
  sm: { padding: `${tokens.spacing.sm} ${tokens.spacing.md}`, fontSize: tokens.typography.fontSize.sm },
  md: { padding: `${tokens.spacing.md} ${tokens.spacing.lg}`, fontSize: tokens.typography.fontSize.base },
  lg: { padding: `${tokens.spacing.md} ${tokens.spacing.xl}`, fontSize: tokens.typography.fontSize.lg },
};

export function OlInput({
  label,
  error,
  size = 'md',
  style,
  id,
  ...rest
}: OlInputProps) {
  const inputId = id ?? (label ? `ol-input-${label.replace(/\s+/g, '-').toLowerCase()}` : undefined);
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs }}>
      {label && (
        <label
          htmlFor={inputId}
          style={{
            fontSize: tokens.typography.fontSize.sm,
            fontWeight: tokens.typography.fontWeight.medium,
            color: tokens.color.neutral['text-primary'],
          }}
        >
          {label}
        </label>
      )}
      <input
        id={inputId}
        style={{
          ...sizeStyles[size],
          width: '100%',
          border: `1px solid ${error ? tokens.color.semantic.error : tokens.color.neutral.border}`,
          borderRadius: tokens.borderRadius.md,
          outline: 'none',
          transition: `border-color var(--ol-motion-fast, 150ms) ease-in-out`,
          background: tokens.color.neutral.bg,
          color: tokens.color.neutral['text-primary'],
          lineHeight: tokens.typography.lineHeight.normal,
          ...style,
        }}
        aria-invalid={error ? true : undefined}
        aria-describedby={error && inputId ? `${inputId}-error` : undefined}
        {...rest}
      />
      {error && (
        <span
          id={inputId ? `${inputId}-error` : undefined}
          role="alert"
          style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.semantic.error }}
        >
          {error}
        </span>
      )}
    </div>
  );
}

interface OlTextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  error?: string;
}

export function OlTextarea({ label, error, style, id, ...rest }: OlTextareaProps) {
  const textareaId = id ?? (label ? `ol-textarea-${label.replace(/\s+/g, '-').toLowerCase()}` : undefined);
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs }}>
      {label && (
        <label
          htmlFor={textareaId}
          style={{
            fontSize: tokens.typography.fontSize.sm,
            fontWeight: tokens.typography.fontWeight.medium,
            color: tokens.color.neutral['text-primary'],
          }}
        >
          {label}
        </label>
      )}
      <textarea
        id={textareaId}
        style={{
          width: '100%',
          padding: tokens.spacing.md,
          fontSize: tokens.typography.fontSize.sm,
          border: `1px solid ${error ? tokens.color.semantic.error : tokens.color.neutral.border}`,
          borderRadius: tokens.borderRadius.lg,
          outline: 'none',
          resize: 'vertical',
          background: tokens.color.neutral.bg,
          color: tokens.color.neutral['text-primary'],
          lineHeight: tokens.typography.lineHeight.normal,
          fontFamily: tokens.typography.fontFamily.base,
          ...style,
        }}
        aria-invalid={error ? true : undefined}
        aria-describedby={error && textareaId ? `${textareaId}-error` : undefined}
        {...rest}
      />
      {error && (
        <span
          id={textareaId ? `${textareaId}-error` : undefined}
          role="alert"
          style={{ fontSize: tokens.typography.fontSize.xs, color: tokens.color.semantic.error }}
        >
          {error}
        </span>
      )}
    </div>
  );
}
