import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { OlErrorBanner, OlLoading, OlPageShell } from '../components/OlErrorBanner';

describe('OlErrorBanner', () => {
  it('renders error message', () => {
    render(<OlErrorBanner message="Something went wrong" />);
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
  });

  it('renders dismiss button when onDismiss provided', () => {
    render(<OlErrorBanner message="Error" onDismiss={() => {}} />);
    expect(screen.getByLabelText('Dismiss')).toBeInTheDocument();
  });

  it('does not render dismiss button when onDismiss not provided', () => {
    render(<OlErrorBanner message="Error" />);
    expect(screen.queryByLabelText('Dismiss')).not.toBeInTheDocument();
  });

  it('renders warning type', () => {
    render(<OlErrorBanner message="Warning" type="warning" />);
    expect(screen.getByText('Warning')).toBeInTheDocument();
  });
});

describe('OlLoading', () => {
  it('renders default loading text', () => {
    render(<OlLoading />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('renders custom loading text', () => {
    render(<OlLoading text="加载中..." />);
    expect(screen.getByText('加载中...')).toBeInTheDocument();
  });
});

describe('OlPageShell', () => {
  it('renders children', () => {
    render(<OlPageShell><div>Content</div></OlPageShell>);
    expect(screen.getByText('Content')).toBeInTheDocument();
  });
});