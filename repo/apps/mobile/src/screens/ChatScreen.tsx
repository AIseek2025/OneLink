import React, { useState, useEffect, useCallback } from 'react';
import { View, Text, TextInput, FlatList, TouchableOpacity, StyleSheet, ActivityIndicator, KeyboardAvoidingView, Platform } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, BffClientError } from '../services/bffClient';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';

interface ChatMessage {
  message_id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  created_at: string;
}

type ChatState = 'ready' | 'sending' | 'reply_loading' | 'reply_streaming' | 'reply_completed' | 'failed';

export function ChatScreen({ conversationId }: { conversationId: string }) {
  const { track } = useAnalytics('ChatScreen');
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputText, setInputText] = useState('');
  const [chatState, setChatState] = useState<ChatState>('ready');
  const [errorMsg, setErrorMsg] = useState('');

  const fetchMessages = useCallback(async () => {
    try {
      const resp = await bffClient.get<{ messages: ChatMessage[] }>(`/chat/conversations/${conversationId}/messages`);
      setMessages(resp.messages || []);
    } catch {
      // silent - show empty chat
    }
  }, [conversationId]);

  useEffect(() => {
    fetchMessages();
  }, [fetchMessages]);

  const handleSend = async () => {
    if (!inputText.trim() || chatState === 'sending') return;
    const userMsg: ChatMessage = {
      message_id: `local-${Date.now()}`,
      role: 'user',
      content: inputText.trim(),
      created_at: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, userMsg]);
    setInputText('');
    setChatState('sending');
    setErrorMsg('');
    track({ event_name: 'chat.message.sent', conversation_id: conversationId, content_type: 'text' });
    try {
      const resp = await bffClient.post<{ message: ChatMessage }>(
        `/chat/conversations/${conversationId}/messages`,
        { content: userMsg.content },
      );
      setMessages((prev) => [...prev, resp.message]);
      setChatState('reply_completed');
      track({ event_name: 'chat.message.received', conversation_id: conversationId });
    } catch (e) {
      setChatState('failed');
      const msg = e instanceof BffClientError ? e.message : '发送失败';
      setErrorMsg(msg);
      track({ event_name: 'error.occurred', error_type: 'runtime', error_message: msg });
    }
  };

  const renderMessage = ({ item }: { item: ChatMessage }) => (
    <View style={[styles.messageBubble, item.role === 'user' ? styles.userBubble : styles.aiBubble]}>
      <Text style={[styles.messageText, item.role === 'user' ? styles.userBubbleText : styles.aiBubbleText]}>
        {item.content}
      </Text>
    </View>
  );

  return (
    <ScreenShell title="Lumi">
      <KeyboardAvoidingView
        style={styles.container}
        behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      >
        {messages.length === 0 ? (
          <View style={styles.centered}>
            <Text style={styles.emptyText}>开始和 Lumi 聊天吧</Text>
          </View>
        ) : (
          <FlatList
            data={messages}
            keyExtractor={(item) => item.message_id}
            renderItem={renderMessage}
            contentContainerStyle={styles.messageList}
            inverted
          />
        )}
        {chatState === 'failed' && (
          <View style={styles.errorBar}>
            <Text style={styles.errorBarText}>{errorMsg}</Text>
            <TouchableOpacity onPress={() => setChatState('ready')}>
              <Text style={styles.dismissText}>关闭</Text>
            </TouchableOpacity>
          </View>
        )}
        {chatState === 'sending' && (
          <View style={styles.typingBar}>
            <ActivityIndicator size="small" color={colors.primary} />
            <Text style={styles.typingText}>Lumi 正在回复…</Text>
          </View>
        )}
        <View style={styles.inputRow}>
          <TextInput
            style={styles.input}
            placeholder="输入消息…"
            placeholderTextColor={colors.textPlaceholder}
            value={inputText}
            onChangeText={setInputText}
            editable={chatState !== 'sending'}
            multiline
          />
          <TouchableOpacity
            style={styles.sendButton}
            onPress={handleSend}
            disabled={!inputText.trim() || chatState === 'sending'}
          >
            <Text style={styles.sendButtonText}>发送</Text>
          </TouchableOpacity>
        </View>
      </KeyboardAvoidingView>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  emptyText: {
    fontSize: typography.fontSize.base,
    color: colors.textSecondary,
  },
  messageList: {
    paddingVertical: spacing.md,
  },
  messageBubble: {
    maxWidth: '80%',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.lg,
    marginVertical: spacing.xs,
    marginHorizontal: spacing.lg,
  },
  userBubble: {
    alignSelf: 'flex-end',
    backgroundColor: colors.chatUserBubble,
  },
  aiBubble: {
    alignSelf: 'flex-start',
    backgroundColor: colors.chatAiBubble,
  },
  userBubbleText: {
    color: colors.chatUserBubbleText,
    fontSize: typography.fontSize.base,
  },
  aiBubbleText: {
    color: colors.chatAiBubbleText,
    fontSize: typography.fontSize.base,
  },
  messageText: {
    lineHeight: 22,
  },
  errorBar: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    backgroundColor: colors.errorBg,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
  },
  errorBarText: {
    color: colors.error,
    fontSize: typography.fontSize.sm,
    flex: 1,
  },
  dismissText: {
    color: colors.error,
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
  },
  typingBar: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.sm,
    gap: spacing.sm,
  },
  typingText: {
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
  },
  inputRow: {
    flexDirection: 'row',
    alignItems: 'flex-end',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    gap: spacing.sm,
    borderTopWidth: 1,
    borderTopColor: colors.border,
  },
  input: {
    flex: 1,
    borderWidth: 1,
    borderColor: colors.border,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    fontSize: typography.fontSize.base,
    color: colors.textPrimary,
    maxHeight: 100,
  },
  sendButton: {
    backgroundColor: colors.primary,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
  },
  sendButtonText: {
    color: colors.chatUserBubbleText,
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
  },
});
