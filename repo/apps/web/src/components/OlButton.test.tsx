import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { OlButton } from '../components/OlButton';

describe('OlButton', () => {
  it('renders children text', () => {
    render(<OlButton>Click me</OlButton>);
    expect(screen.getByText('Click me')).toBeInTheDocument();
  });

  it('applies primary variant by default', () => {
    render(<OlButton>Primary</OlButton>);
    const btn = screen.getByText('Primary');
    expect(btn).toBeInTheDocument();
    expect(btn.tagName).toBe('BUTTON');
  });

  it('shows spinner when loading', () => {
    render(<OlButton loading>Loading</OlButton>);
    const btn = screen.getByText('Loading').closest('button')!;
    expect(btn.disabled).toBe(true);
  });

  it('is disabled when disabled prop is true', () => {
    render(<OlButton disabled>Disabled</OlButton>);
    const btn = screen.getByText('Disabled').closest('button')!;
    expect(btn.disabled).toBe(true);
  });

  it('applies fullWidth style', () => {
    render(<OlButton fullWidth>Full</OlButton>);
    const btn = screen.getByText('Full').closest('button')!;
    expect(btn.style.width).toBe('100%');
  });
});