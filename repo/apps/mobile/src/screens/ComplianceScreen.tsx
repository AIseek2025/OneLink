import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { View, Text, ScrollView, TouchableOpacity, TextInput, StyleSheet, ActivityIndicator } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { fetchComplianceData, requestComplianceAction } from '../services/api';
import { BffClientError } from '../services/bffClient';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';

type ComplianceState = 'loading' | 'ready' | 'failed';

interface ComplianceData {
  user_id?: string;
  email?: string;
  display_name?: string;
  primary_region?: string;
  primary_language?: string;
  created_at?: string;
  recommendations_count?: number;
  dm_threads_count?: number;
}

export function ComplianceScreen() {
  const { track } = useAnalytics('ComplianceScreen');
  const [state, setState] = useState<ComplianceState>('loading');
  const [data, setData] = useState<ComplianceData | null>(null);
  const [errorMsg, setErrorMsg] = useState('');
  const [successMsg, setSuccessMsg] = useState('');
  const [fieldName, setFieldName] = useState('');
  const [scope, setScope] = useState('account');
  const [submitting, setSubmitting] = useState<'export' | 'correction' | 'delete' | null>(null);

  const facts = useMemo(
    () =>
      [
        ['用户 ID', data?.user_id ?? '未返回'],
        ['邮箱', data?.email ?? '未返回'],
        ['昵称', data?.display_name ?? '未返回'],
        ['地区', data?.primary_region ?? '未返回'],
        ['语言', data?.primary_language ?? '未返回'],
        ['注册时间', data?.created_at ?? '未返回'],
        ['推荐次数', String(data?.recommendations_count ?? 0)],
        ['私信线程数', String(data?.dm_threads_count ?? 0)],
      ] as const,
    [data],
  );

  const loadData = useCallback(async () => {
    setState('loading');
    setErrorMsg('');
    try {
      const response = await fetchComplianceData();
      setData((response as ComplianceData) ?? null);
      setState('ready');
      track({ event_name: 'data_rights_view' });
    } catch (error) {
      setState('failed');
      setErrorMsg(error instanceof BffClientError ? error.message : '加载合规数据失败');
    }
  }, [track]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleAction = async (action: 'export' | 'correction' | 'delete') => {
    setSubmitting(action);
    setErrorMsg('');
    setSuccessMsg('');
    try {
      await requestComplianceAction({
        action_type: action,
        scope,
        field_name: action === 'correction' ? fieldName || 'profile' : undefined,
        export_format: action === 'export' ? 'json' : undefined,
      });
      if (action === 'export') {
        track({ event_name: 'data_export_request' });
        setSuccessMsg('已提交数据导出申请');
      } else if (action === 'correction') {
        track({ event_name: 'data_correction_request' });
        setSuccessMsg(`已提交字段更正申请: ${fieldName || 'profile'}`);
      } else {
        track({ event_name: 'data_delete_request' });
        setSuccessMsg('已提交删除申请');
      }
    } catch (error) {
      setErrorMsg(error instanceof BffClientError ? error.message : '提交合规操作失败');
    } finally {
      setSubmitting(null);
    }
  };

  if (state === 'loading') {
    return (
      <ScreenShell title="数据合规">
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={colors.primary} />
        </View>
      </ScreenShell>
    );
  }

  if (state === 'failed') {
    return (
      <ScreenShell title="数据合规">
        <View style={styles.centered}>
          <Text style={styles.errorText}>{errorMsg}</Text>
          <TouchableOpacity style={styles.primaryButton} onPress={loadData}>
            <Text style={styles.primaryButtonText}>重试</Text>
          </TouchableOpacity>
        </View>
      </ScreenShell>
    );
  }

  return (
    <ScreenShell title="数据合规">
      <ScrollView contentContainerStyle={styles.content}>
        {errorMsg ? (
          <View style={styles.errorBanner}>
            <Text style={styles.errorBannerText}>{errorMsg}</Text>
          </View>
        ) : null}
        {successMsg ? (
          <View style={styles.successBanner}>
            <Text style={styles.successBannerText}>{successMsg}</Text>
          </View>
        ) : null}

        <Text style={styles.sectionTitle}>个人数据概览</Text>
        <View style={styles.card}>
          {facts.map(([label, value]) => (
            <View key={label} style={styles.factRow}>
              <Text style={styles.factLabel}>{label}</Text>
              <Text style={styles.factValue}>{value}</Text>
            </View>
          ))}
        </View>

        <Text style={styles.sectionTitle}>数据权利操作</Text>
        <View style={styles.card}>
          <Text style={styles.fieldLabel}>作用范围</Text>
          <TextInput
            style={styles.input}
            value={scope}
            onChangeText={setScope}
            placeholder="account"
            placeholderTextColor={colors.textPlaceholder}
          />
          <Text style={styles.fieldHint}>可填写 `account`、`profile` 或更精细的数据范围。</Text>

          <TouchableOpacity
            style={[styles.primaryButton, submitting ? styles.buttonDisabled : null]}
            onPress={() => handleAction('export')}
            disabled={Boolean(submitting)}
          >
            <Text style={styles.primaryButtonText}>{submitting === 'export' ? '提交中...' : '申请导出数据'}</Text>
          </TouchableOpacity>

          <Text style={styles.fieldLabel}>更正字段</Text>
          <TextInput
            style={styles.input}
            value={fieldName}
            onChangeText={setFieldName}
            placeholder="例如 display_name"
            placeholderTextColor={colors.textPlaceholder}
          />
          <TouchableOpacity
            style={[styles.secondaryButton, submitting ? styles.buttonDisabled : null]}
            onPress={() => handleAction('correction')}
            disabled={Boolean(submitting)}
          >
            <Text style={styles.secondaryButtonText}>{submitting === 'correction' ? '提交中...' : '申请更正字段'}</Text>
          </TouchableOpacity>

          <TouchableOpacity
            style={[styles.dangerButton, submitting ? styles.buttonDisabled : null]}
            onPress={() => handleAction('delete')}
            disabled={Boolean(submitting)}
          >
            <Text style={styles.dangerButtonText}>{submitting === 'delete' ? '提交中...' : '申请删除数据'}</Text>
          </TouchableOpacity>
        </View>
      </ScrollView>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  content: {
    paddingBottom: spacing.xxl,
  },
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: spacing.xl,
  },
  sectionTitle: {
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
    color: colors.textSecondary,
    textTransform: 'uppercase',
    marginTop: spacing.lg,
    marginBottom: spacing.md,
  },
  card: {
    backgroundColor: colors.backgroundSecondary,
    borderRadius: borderRadius.lg,
    padding: spacing.lg,
    gap: spacing.md,
  },
  factRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    gap: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
    paddingBottom: spacing.sm,
  },
  factLabel: {
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
  },
  factValue: {
    flex: 1,
    fontSize: typography.fontSize.sm,
    color: colors.textPrimary,
    textAlign: 'right',
  },
  fieldLabel: {
    fontSize: typography.fontSize.base,
    fontWeight: typography.fontWeight.medium,
    color: colors.textPrimary,
  },
  fieldHint: {
    fontSize: typography.fontSize.xs,
    color: colors.textSecondary,
    marginTop: -spacing.xs,
  },
  input: {
    borderWidth: 1,
    borderColor: colors.border,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    fontSize: typography.fontSize.base,
    color: colors.textPrimary,
    backgroundColor: colors.background,
  },
  primaryButton: {
    backgroundColor: colors.primary,
    borderRadius: borderRadius.md,
    paddingVertical: spacing.md,
    alignItems: 'center',
  },
  primaryButtonText: {
    color: colors.chatUserBubbleText,
    fontSize: typography.fontSize.base,
    fontWeight: typography.fontWeight.semibold,
  },
  secondaryButton: {
    backgroundColor: colors.surface,
    borderRadius: borderRadius.md,
    paddingVertical: spacing.md,
    alignItems: 'center',
    borderWidth: 1,
    borderColor: colors.border,
  },
  secondaryButtonText: {
    color: colors.textPrimary,
    fontSize: typography.fontSize.base,
    fontWeight: typography.fontWeight.semibold,
  },
  dangerButton: {
    backgroundColor: colors.errorBg,
    borderRadius: borderRadius.md,
    paddingVertical: spacing.md,
    alignItems: 'center',
  },
  dangerButtonText: {
    color: colors.error,
    fontSize: typography.fontSize.base,
    fontWeight: typography.fontWeight.semibold,
  },
  buttonDisabled: {
    opacity: 0.7,
  },
  errorText: {
    fontSize: typography.fontSize.base,
    color: colors.error,
    textAlign: 'center',
    marginBottom: spacing.md,
  },
  errorBanner: {
    backgroundColor: colors.errorBg,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    marginBottom: spacing.md,
  },
  errorBannerText: {
    color: colors.error,
    fontSize: typography.fontSize.sm,
  },
  successBanner: {
    backgroundColor: colors.successBg,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    marginBottom: spacing.md,
  },
  successBannerText: {
    color: colors.success,
    fontSize: typography.fontSize.sm,
  },
});
