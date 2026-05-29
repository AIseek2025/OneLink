import React from 'react';
import { View, Text, StyleSheet } from 'react-native';
import { colors, spacing, typography } from '../theme/tokens';

type StateOverlay = 'empty' | 'loading' | 'error' | 'degraded';

interface ScreenShellProps {
  title: string;
  state?: StateOverlay;
  errorMessage?: string;
  children?: React.ReactNode;
}

const stateLabels: Record<StateOverlay, string> = {
  empty: '暂无数据',
  loading: '加载中…',
  error: '加载失败，请重试',
  degraded: '部分数据不可用',
};

export function ScreenShell({ title, state, errorMessage, children }: ScreenShellProps) {
  return (
    <View style={styles.container}>
      <Text style={styles.title}>{title}</Text>
      {state && (
        <View style={styles.stateContainer}>
          <Text
            style={[
              styles.stateText,
              state === 'error' && styles.errorText,
            ]}
          >
            {state === 'error' && errorMessage ? errorMessage : stateLabels[state]}
          </Text>
        </View>
      )}
      {children}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
    paddingHorizontal: spacing.lg,
    paddingTop: spacing.xl,
  },
  title: {
    fontSize: typography.fontSize.xxl,
    fontWeight: typography.fontWeight.bold,
    color: colors.textPrimary,
    marginBottom: spacing.md,
  },
  stateContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  stateText: {
    fontSize: typography.fontSize.base,
    color: colors.textSecondary,
  },
  errorText: {
    color: colors.error,
  },
});
