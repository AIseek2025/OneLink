import { Component, type ReactNode, type ErrorInfo } from 'react';
import { tokens } from '../design-tokens';

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: (error: Error, reset: () => void) => ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class OlErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('[OlErrorBoundary]', error, info);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError && this.state.error) {
      if (this.props.fallback) {
        return this.props.fallback(this.state.error, this.handleReset);
      }
      return (
        <div
          role="alert"
          style={{
            padding: tokens.spacing.xl,
            background: tokens.color.semantic['error-bg'],
            borderRadius: tokens.borderRadius.lg,
            border: `1px solid ${tokens.color.semantic.error}`,
            maxWidth: 640,
            margin: `${tokens.spacing.xl} auto`,
          }}
        >
          <h3 style={{ color: tokens.color.semantic.error, margin: `0 0 ${tokens.spacing.md} 0` }}>
            页面出现错误
          </h3>
          <p style={{ color: tokens.color.neutral['text-primary'], fontSize: tokens.typography.fontSize.sm, margin: `0 0 ${tokens.spacing.lg} 0` }}>
            {this.state.error.message}
          </p>
          <button
            onClick={this.handleReset}
            style={{
              padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`,
              background: tokens.color.brand.primary,
              color: '#FFFFFF',
              border: 'none',
              borderRadius: tokens.borderRadius.md,
              cursor: 'pointer',
              fontSize: tokens.typography.fontSize.sm,
            }}
          >
            重试
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}