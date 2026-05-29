import React, { useEffect, useState, useCallback } from 'react';
import { View, Text, FlatList, TouchableOpacity, StyleSheet, RefreshControl, ActivityIndicator } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, BffClientError } from '../services/bffClient';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';

interface DmThread {
  thread_id: string;
  recipient_nickname: string;
  last_message_preview?: string;
  updated_at: string;
}

type MsgState = 'loading' | 'empty' | 'ready' | 'failed';

export function MessagesScreen() {
  const { track } = useAnalytics('MessagesScreen');
  const [threads, setThreads] = useState<DmThread[]>([]);
  const [msgState, setMsgState] = useState<MsgState>('loading');
  const [refreshing, setRefreshing] = useState(false);
  const [errorMsg, setErrorMsg] = useState('');

  const fetchThreads = useCallback(async () => {
    setMsgState('loading');
    try {
      const resp = await bffClient.get<{ threads: DmThread[] }>('/dm/threads');
      setThreads(resp.threads || []);
      setMsgState(resp.threads?.length ? 'ready' : 'empty');
    } catch (e) {
      setMsgState('failed');
      setErrorMsg(e instanceof BffClientError ? e.message : '加载失败');
    }
  }, []);

  useEffect(() => {
    fetchThreads();
  }, [fetchThreads]);

  const renderItem = ({ item }: { item: DmThread }) => (
    <TouchableOpacity
      style={styles.threadItem}
      onPress={() => track({ event_name: 'conversation_open', conversation_id: item.thread_id })}
    >
      <Text style={styles.threadNickname}>{item.recipient_nickname}</Text>
      {item.last_message_preview ? (
        <Text style={styles.threadPreview} numberOfLines={1}>{item.last_message_preview}</Text>
      ) : null}
    </TouchableOpacity>
  );

  return (
    <ScreenShell title="消息">
      {msgState === 'loading' && (
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={colors.primary} />
        </View>
      )}
      {msgState === 'empty' && (
        <View style={styles.centered}>
          <Text style={styles.emptyText}>暂无消息</Text>
        </View>
      )}
      {msgState === 'failed' && (
        <View style={styles.centered}>
          <Text style={styles.errorText}>{errorMsg}</Text>
          <TouchableOpacity style={styles.retryButton} onPress={fetchThreads}>
            <Text style={styles.retryButtonText}>重试</Text>
          </TouchableOpacity>
        </View>
      )}
      {msgState === 'ready' && (
        <FlatList
          data={threads}
          keyExtractor={(item) => item.thread_id}
          renderItem={renderItem}
          refreshControl={<RefreshControl refreshing={refreshing} onRefresh={async () => { setRefreshing(true); await fetchThreads(); setRefreshing(false); }} />}
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
  threadItem: {
    paddingVertical: spacing.md,
    paddingHorizontal: spacing.lg,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  threadNickname: {
    fontSize: typography.fontSize.base,
    fontWeight: typography.fontWeight.medium,
    color: colors.textPrimary,
  },
  threadPreview: {
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
    marginTop: spacing.xs,
  },
});
