import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { OlChatBubble } from '../components/OlChatBubble';

describe('OlChatBubble', () => {
  it('renders user message', () => {
    render(<OlChatBubble message={{ id: '1', text: 'Hello', sender: 'user' }} />);
    expect(screen.getByText('Hello')).toBeInTheDocument();
  });

  it('renders AI message', () => {
    render(<OlChatBubble message={{ id: '2', text: 'Hi there', sender: 'ai' }} />);
    expect(screen.getByText('Hi there')).toBeInTheDocument();
  });
});