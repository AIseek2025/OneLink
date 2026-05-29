import React, { useEffect, useState, useCallback } from 'react';
import { View, Text, FlatList, TouchableOpacity, StyleSheet, RefreshControl, ActivityIndicator } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, BffClientError } from '../services/bffClient';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';
import type { ConversationSummary } from '../types';

type ListState = 'loading_list' | 'empty_list' | 'list_ready' | 'load_failed';

export function HomeScreen() {
  const { track } = useAnalytics('HomeScreen');
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [listState, setListState] = useState<ListState>('loading_list');
  const [refreshing, setRefreshing] = useState(false);
  const [errorMsg, setErrorMsg] = useState('');

  const fetchConversations = useCallback(async () => {
    setListState('loading_list');
    try {
      const resp = await bffClient.get<{ conversations: ConversationSummary[] }>('/chat/conversations');
      setConversations(resp.conversations || []);
      setListState(resp.conversations?.length ? 'list_ready' : 'empty_list');
    } catch (e) {
      setListState('load_failed');
      setErrorMsg(e instanceof BffClientError ? e.message : '加载失败');
    }
  }, []);

  useEffect(() => {
    fetchConversations();
  }, [fetchConversations]);

  const renderItem = ({ item }: { item: ConversationSummary }) => (
    <TouchableOpacity
      style={styles.conversationItem}
      onPress={() => track({ event_name: 'conversation_open', conversation_id: item.conversation_id })}
    >
      <Text style={styles.conversationTitle}>{item.title}</Text>
      {item.last_message_preview ? (
        <Text style={styles.conversationPreview} numberOfLines={1}>
          {item.last_message_preview}
        </Text>
      ) : null}
    </TouchableOpacity>
  );

  return (
    <ScreenShell title="首页 / Lumi">
      {listState === 'loading_list' && (
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={colors.primary} />
        </View>
      )}
      {listState === 'empty_list' && (
        <View style={styles.centered}>
          <Text style={styles.emptyText}>还没有对话，开始和 Lumi 聊天吧</Text>
        </View>
      )}
      {listState === 'load_failed' && (
        <View style={styles.centered}>
          <Text style={styles.errorText}>{errorMsg}</Text>
          <TouchableOpacity style={styles.retryButton} onPress={fetchConversations}>
            <Text style={styles.retryButtonText}>重试</Text>
          </TouchableOpacity>
        </View>
      )}
      {listState === 'list_ready' && (
        <FlatList
          data={conversations}
          keyExtractor={(item) => item.conversation_id}
          renderItem={renderItem}
          refreshControl={<RefreshControl refreshing={refreshing} onRefresh={async () => { setRefreshing(true); await fetchConversations(); setRefreshing(false); }} />}
        />
      )}
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: spacing.xl,
  },
  emptyText: {
    fontSize: typography.fontSize.base,
    color: colors.textSecondary,
    textAlign: 'center',
  },
  errorText: {
    fontSize: typography.fontSize.base,
    color: colors.error,
    textAlign: 'center',
    marginBottom: spacing.md,
  },
  retryButton: {
    backgroundColor: colors.primary,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.xl,
    paddingVertical: spacing.sm,
  },
  retryButtonText: {
    color: colors.chatUserBubbleText,
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
  },
  conversationItem: {
    paddingVertical: spacing.md,
    paddingHorizontal: spacing.lg,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  conversationTitle: {
    fontSize: typography.fontSize.base,
    fontWeight: typography.fontWeight.medium,
    color: colors.textPrimary,
  },
  conversationPreview: {
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
    marginTop: spacing.xs,
  },
});
