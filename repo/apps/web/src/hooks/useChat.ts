import { useState, useCallback, useRef, useEffect } from 'react';
import { fetchChatInit, sendMessage } from '../api/client';
import { trackEvent, getAnalyticsContext } from '../analytics';

interface ChatInitData {
  user: { user_id: string };
  conversation: { conversation_id: string; status: string };
  pending_questions: Array<{ delivery_id: string; question_text: string }>;
}

interface Message {
  id: string;
  text: string;
  sender: 'user' | 'ai';
}

export function useChat() {
  const [initData, setInitData] = useState<ChatInitData | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSending, setIsSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, []);

  useEffect(() => {
    const token = localStorage.getItem('onelink_token');
    if (!token) { setIsLoading(false); return; }
    fetchChatInit()
      .then((d) => {
        setInitData(d);
        setIsLoading(false);
        trackEvent(
          { event_name: 'page.view', page_path: '/chat' },
          getAnalyticsContext({ screen: '/chat', user_id: d.user?.user_id ?? null }),
        );
      })
      .catch((e) => {
        setError(e.message);
        setIsLoading(false);
        trackEvent(
          { event_name: 'error.occurred', error_type: 'network', error_message: e.message },
          getAnalyticsContext({ screen: '/chat' }),
        );
      });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, scrollToBottom]);

  const send = useCallback(async () => {
    if (!input.trim() || !initData || isSending) return;
    const userMsg: Message = { id: Date.now().toString(), text: input, sender: 'user' };
    setMessages((prev) => [...prev, userMsg]);
    const sentAt = Date.now();
    const text = input;
    setInput('');
    setIsSending(true);
    try {
      const res = await sendMessage(initData.conversation.conversation_id, text);
      const aiMsg: Message = { id: res.ai_message_id, text: res.ai_content_text, sender: 'ai' };
      setMessages((prev) => [...prev, aiMsg]);
      const ctx = getAnalyticsContext({ screen: '/chat', user_id: initData.user?.user_id ?? null });
      trackEvent(
        { event_name: 'chat.message.sent', conversation_id: initData.conversation.conversation_id, content_type: 'text' },
        ctx,
      );
      trackEvent(
        { event_name: 'chat.message.received', conversation_id: initData.conversation.conversation_id, response_latency_ms: Date.now() - sentAt },
        ctx,
      );
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Send failed';
      setError(msg);
      trackEvent(
        { event_name: 'error.occurred', error_type: 'network', error_message: msg },
        getAnalyticsContext({ screen: '/chat' }),
      );
    } finally {
      setIsSending(false);
    }
  }, [input, initData, isSending]);

  const clearError = useCallback(() => setError(null), []);

  return {
    initData,
    messages,
    input,
    setInput,
    isLoading,
    isSending,
    error,
    send,
    clearError,
    messagesEndRef,
    pendingQuestions: initData?.pending_questions ?? [],
  };
}