import { tokens } from '../design-tokens';

interface Message {
  id: string;
  text: string;
  sender: 'user' | 'ai';
}

interface OlChatBubbleProps {
  message: Message;
}

export function OlChatBubble({ message }: OlChatBubbleProps) {
  const isUser = message.sender === 'user';
  return (
    <div
      style={{
        marginBottom: tokens.spacing.sm,
        textAlign: isUser ? 'right' : 'left',
      }}
    >
      <span
        style={{
          display: 'inline-block',
          padding: `${tokens.spacing.md} ${tokens.spacing.lg}`,
          borderRadius: tokens.borderRadius.lg,
          background: isUser ? tokens.color.chat['user-bubble'] : tokens.color.chat['ai-bubble'],
          color: isUser ? tokens.color.chat['user-bubble-text'] : tokens.color.chat['ai-bubble-text'],
          fontSize: tokens.typography.fontSize.sm,
          lineHeight: tokens.typography.lineHeight.normal,
          maxWidth: '80%',
          wordBreak: 'break-word',
        }}
      >
        {message.text}
      </span>
    </div>
  );
}

interface OlChatWindowProps {
  messages: Message[];
  input: string;
  onInputChange: (value: string) => void;
  onSend: () => void;
  placeholder?: string;
  loading?: boolean;
  emptyText?: string;
}

export function OlChatWindow({
  messages,
  input,
  onInputChange,
  onSend,
  placeholder = '输入消息...',
  loading = false,
  emptyText = '和 Lumi 开始对话吧',
}: OlChatWindowProps) {
  return (
    <div
      style={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
      }}
    >
      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          border: `1px solid ${tokens.color.neutral.border}`,
          borderRadius: tokens.borderRadius.lg,
          padding: tokens.spacing.md,
          marginBottom: tokens.spacing.md,
        }}
      >
        {messages.length === 0 && (
          <p style={{ color: tokens.color.neutral['text-placeholder'], textAlign: 'center' }}>
            {emptyText}
          </p>
        )}
        {messages.map((m) => (
          <OlChatBubble key={m.id} message={m} />
        ))}
      </div>
      <div style={{ display: 'flex', gap: tokens.spacing.sm }}>
        <input
          value={input}
          onChange={(e) => onInputChange(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && onSend()}
          placeholder={placeholder}
          disabled={loading}
          style={{
            flex: 1,
            padding: tokens.spacing.md,
            fontSize: tokens.typography.fontSize.sm,
            border: `1px solid ${tokens.color.neutral.border}`,
            borderRadius: tokens.borderRadius.md,
            outline: 'none',
            background: tokens.color.neutral.bg,
          }}
        />
        <button
          onClick={onSend}
          disabled={loading}
          style={{
            padding: `${tokens.spacing.md} ${tokens.spacing.lg}`,
            background: tokens.color.brand.primary,
            color: '#FFFFFF',
            border: 'none',
            borderRadius: tokens.borderRadius.md,
            cursor: loading ? 'not-allowed' : 'pointer',
            fontWeight: tokens.typography.fontWeight.semibold,
            opacity: loading ? 0.5 : 1,
          }}
        >
          发送
        </button>
      </div>
    </div>
  );
}