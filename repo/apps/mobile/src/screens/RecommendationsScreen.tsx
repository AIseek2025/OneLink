import React, { useEffect, useState, useCallback } from 'react';
import { View, Text, FlatList, TouchableOpacity, StyleSheet, RefreshControl, ActivityIndicator } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, BffClientError } from '../services/bffClient';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';
import type { Recommendation } from '../types';

type RecState = 'waiting_results' | 'results_ready' | 'empty_result' | 'failed';

export function RecommendationsScreen() {
  const { track } = useAnalytics('RecommendationsScreen');
  const [recommendations, setRecommendations] = useState<Recommendation[]>([]);
  const [recState, setRecState] = useState<RecState>('waiting_results');
  const [refreshing, setRefreshing] = useState(false);
  const [errorMsg, setErrorMsg] = useState('');

  const fetchRecommendations = useCallback(async () => {
    setRecState('waiting_results');
    try {
      const resp = await bffClient.get<{ recommendations: Recommendation[] }>('/recommendations');
      setRecommendations(resp.recommendations || []);
      setRecState(resp.recommendations?.length ? 'results_ready' : 'empty_result');
    } catch (e) {
      setRecState('failed');
      setErrorMsg(e instanceof BffClientError ? e.message : '加载失败');
    }
  }, []);

  useEffect(() => {
    fetchRecommendations();
  }, [fetchRecommendations]);

  const handleFeedback = async (recId: string, feedbackType: string) => {
    track({ event_name: 'recommendation_feedback_submit', recommendation_id: recId, feedback_type: feedbackType });
    try {
      await bffClient.post(`/recommendations/${recId}/feedback`, { feedback_type: feedbackType });
    } catch {
      // feedback errors are non-blocking for UX
    }
  };

  const renderItem = ({ item }: { item: Recommendation }) => (
    <View style={styles.card}>
      <Text style={styles.cardNickname}>{item.nickname}</Text>
      <Text style={styles.cardReason} numberOfLines={2}>{item.reason}</Text>
      <View style={styles.feedbackRow}>
        <TouchableOpacity
          style={[styles.feedbackButton, styles.likeButton]}
          onPress={() => handleFeedback(item.recommendation_id, 'like')}
        >
          <Text style={styles.likeButtonText}>喜欢</Text>
        </TouchableOpacity>
        <TouchableOpacity
          style={[styles.feedbackButton, styles.skipButton]}
          onPress={() => handleFeedback(item.recommendation_id, 'skip')}
        >
          <Text style={styles.skipButtonText}>跳过</Text>
        </TouchableOpacity>
        <TouchableOpacity
          style={[styles.feedbackButton, styles.laterButton]}
          onPress={() => handleFeedback(item.recommendation_id, 'later')}
        >
          <Text style={styles.laterButtonText}>稍后看</Text>
        </TouchableOpacity>
      </View>
    </View>
  );

  return (
    <ScreenShell title="推荐">
      {recState === 'waiting_results' && (
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={colors.primary} />
        </View>
      )}
      {recState === 'empty_result' && (
        <View style={styles.centered}>
          <Text style={styles.emptyText}>暂无推荐，请先提交找人请求</Text>
          <TouchableOpacity style={styles.retryButton} onPress={fetchRecommendations}>
            <Text style={styles.retryButtonText}>刷新</Text>
          </TouchableOpacity>
        </View>
      )}
      {recState === 'failed' && (
        <View style={styles.centered}>
          <Text style={styles.errorText}>{errorMsg}</Text>
          <TouchableOpacity style={styles.retryButton} onPress={fetchRecommendations}>
            <Text style={styles.retryButtonText}>重试</Text>
          </TouchableOpacity>
        </View>
      )}
      {recState === 'results_ready' && (
        <FlatList
          data={recommendations}
          keyExtractor={(item) => item.recommendation_id}
          renderItem={renderItem}
          refreshControl={<RefreshControl refreshing={refreshing} onRefresh={async () => { setRefreshing(true); await fetchRecommendations(); setRefreshing(false); }} />}
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
    marginBottom: spacing.md,
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
  card: {
    backgroundColor: colors.backgroundSecondary,
    borderRadius: borderRadius.lg,
    padding: spacing.lg,
    marginHorizontal: spacing.lg,
    marginVertical: spacing.sm,
  },
  cardNickname: {
    fontSize: typography.fontSize.lg,
    fontWeight: typography.fontWeight.semibold,
    color: colors.textPrimary,
    marginBottom: spacing.xs,
  },
  cardReason: {
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
    marginBottom: spacing.md,
  },
  feedbackRow: {
    flexDirection: 'row',
    gap: spacing.sm,
  },
  feedbackButton: {
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
  },
  likeButton: {
    backgroundColor: colors.primary,
  },
  likeButtonText: {
    color: colors.chatUserBubbleText,
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
  },
  skipButton: {
    backgroundColor: colors.surface,
    borderWidth: 1,
    borderColor: colors.border,
  },
  skipButtonText: {
    color: colors.textSecondary,
    fontSize: typography.fontSize.sm,
  },
  laterButton: {
    backgroundColor: colors.surface,
  },
  laterButtonText: {
    color: colors.info,
    fontSize: typography.fontSize.sm,
  },
});
