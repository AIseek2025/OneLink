import React, { useEffect, useState, useCallback } from 'react';
import { View, Text, FlatList, TouchableOpacity, StyleSheet, ActivityIndicator, RefreshControl } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, BffClientError } from '../services/bffClient';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';

interface ProfileFact {
  fact_key: string;
  fact_value: string;
  confidence: string;
  source: string;
  confirmed: boolean;
}

type ProfileState = 'loading' | 'empty' | 'ready' | 'failed';

export function ProfileConfirmationScreen({ onBack }: { onBack?: () => void }) {
  const { track } = useAnalytics('ProfileConfirmationScreen');
  const [facts, setFacts] = useState<ProfileFact[]>([]);
  const [profileState, setProfileState] = useState<ProfileState>('loading');
  const [refreshing, setRefreshing] = useState(false);
  const [errorMsg, setErrorMsg] = useState('');

  const fetchFacts = useCallback(async () => {
    setProfileState('loading');
    try {
      const resp = await bffClient.get<{ facts: ProfileFact[] }>('/users/me/facts');
      setFacts(resp.facts || []);
      setProfileState(resp.facts?.length ? 'ready' : 'empty');
      track({ event_name: 'profile.confirmation.viewed', completion_rate: resp.facts?.length ?? 0 });
    } catch (e) {
      setProfileState('failed');
      setErrorMsg(e instanceof BffClientError ? e.message : '加载失败');
    }
  }, [track]);

  useEffect(() => {
    fetchFacts();
  }, [fetchFacts]);

  const handleConfirm = async (factKey: string) => {
    track({ event_name: 'profile.fact.confirmed', fact_type: factKey, fact_value: '' });
    try {
      await bffClient.post(`/users/me/facts/${factKey}/confirm`, {});
      setFacts((prev) => prev.map((f) => (f.fact_key === factKey ? { ...f, confirmed: true } : f)));
    } catch {
      setErrorMsg('确认失败');
    }
  };

  const handleDismiss = async (factKey: string) => {
    track({ event_name: 'profile.fact.dismissed', fact_type: factKey, fact_value: '' });
    try {
      await bffClient.post(`/users/me/facts/${factKey}/dismiss`, {});
      setFacts((prev) => prev.filter((f) => f.fact_key !== factKey));
    } catch {
      setErrorMsg('忽略失败');
    }
  };

  const renderItem = ({ item }: { item: ProfileFact }) => (
    <View style={styles.factCard}>
      <View style={styles.factHeader}>
        <Text style={styles.factKey}>{item.fact_key}</Text>
        <Text style={[styles.confidenceBadge, item.confidence === 'high' ? styles.highConfidence : styles.lowConfidence]}>
          {item.confidence}
        </Text>
      </View>
      <Text style={styles.factValue}>{item.fact_value}</Text>
      <Text style={styles.factSource}>来源: {item.source}</Text>
      {item.confirmed ? (
        <View style={styles.confirmedBadge}>
          <Text style={styles.confirmedText}>已确认</Text>
        </View>
      ) : (
        <View style={styles.actionRow}>
          <TouchableOpacity style={styles.confirmButton} onPress={() => handleConfirm(item.fact_key)}>
            <Text style={styles.confirmButtonText}>确认</Text>
          </TouchableOpacity>
          <TouchableOpacity style={styles.dismissButton} onPress={() => handleDismiss(item.fact_key)}>
            <Text style={styles.dismissButtonText}>忽略</Text>
          </TouchableOpacity>
        </View>
      )}
    </View>
  );

  return (
    <ScreenShell title="画像确认">
      {profileState === 'loading' && (
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={colors.primary} />
        </View>
      )}
      {profileState === 'empty' && (
        <View style={styles.centered}>
          <Text style={styles.emptyText}>暂无待确认的画像信息</Text>
          {onBack && (
            <TouchableOpacity style={styles.backButton} onPress={onBack}>
              <Text style={styles.backButtonText}>返回</Text>
            </TouchableOpacity>
          )}
        </View>
      )}
      {profileState === 'failed' && (
        <View style={styles.centered}>
          <Text style={styles.errorText}>{errorMsg}</Text>
          <TouchableOpacity style={styles.retryButton} onPress={fetchFacts}>
            <Text style={styles.retryButtonText}>重试</Text>
          </TouchableOpacity>
        </View>
      )}
      {profileState === 'ready' && (
        <FlatList
          data={facts}
          keyExtractor={(item) => item.fact_key}
          renderItem={renderItem}
          refreshControl={<RefreshControl refreshing={refreshing} onRefresh={async () => { setRefreshing(true); await fetchFacts(); setRefreshing(false); }} />}
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
  backButton: {
    backgroundColor: colors.surface,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.xl,
    paddingVertical: spacing.sm,
    marginTop: spacing.md,
  },
  backButtonText: {
    color: colors.textPrimary,
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
  },
  factCard: {
    backgroundColor: colors.backgroundSecondary,
    borderRadius: borderRadius.lg,
    padding: spacing.lg,
    marginHorizontal: spacing.lg,
    marginVertical: spacing.sm,
  },
  factHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: spacing.xs,
  },
  factKey: {
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
    color: colors.textSecondary,
    textTransform: 'uppercase',
  },
  confidenceBadge: {
    borderRadius: borderRadius.sm,
    paddingHorizontal: spacing.sm,
    paddingVertical: 2,
    fontSize: typography.fontSize.xs,
    fontWeight: typography.fontWeight.medium,
  },
  highConfidence: {
    backgroundColor: colors.successBg,
    color: colors.success,
  },
  lowConfidence: {
    backgroundColor: colors.warningBg,
    color: colors.warning,
  },
  factValue: {
    fontSize: typography.fontSize.lg,
    fontWeight: typography.fontWeight.medium,
    color: colors.textPrimary,
    marginBottom: spacing.xs,
  },
  factSource: {
    fontSize: typography.fontSize.xs,
    color: colors.textSecondary,
    marginBottom: spacing.md,
  },
  confirmedBadge: {
    backgroundColor: colors.successBg,
    borderRadius: borderRadius.md,
    paddingVertical: spacing.xs,
    paddingHorizontal: spacing.md,
    alignSelf: 'flex-start',
  },
  confirmedText: {
    color: colors.success,
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
  },
  actionRow: {
    flexDirection: 'row',
    gap: spacing.sm,
  },
  confirmButton: {
    backgroundColor: colors.primary,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.sm,
  },
  confirmButtonText: {
    color: colors.chatUserBubbleText,
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
  },
  dismissButton: {
    backgroundColor: colors.surface,
    borderWidth: 1,
    borderColor: colors.border,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.sm,
  },
  dismissButtonText: {
    color: colors.textSecondary,
    fontSize: typography.fontSize.sm,
  },
});
