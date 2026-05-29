import React, { useEffect, useState, useCallback } from 'react';
import { View, Text, TouchableOpacity, StyleSheet, ActivityIndicator, ScrollView } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, setAuthToken, BffClientError } from '../services/bffClient';
import { clearAuthTokens } from '../services/config';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';
import type { SettingsDto, User } from '../types';

type MeState = 'loading' | 'ready' | 'failed';

export function MeScreen({ onLogout, onNavigateSettings, onNavigateProfile }: { onLogout: () => void; onNavigateSettings?: () => void; onNavigateProfile?: () => void }) {
  const { track } = useAnalytics('MeScreen');
  const [meState, setMeState] = useState<MeState>('loading');
  const [user, setUser] = useState<User | null>(null);
  const [settings, setSettings] = useState<SettingsDto | null>(null);
  const [errorMsg, setErrorMsg] = useState('');

  const fetchProfile = useCallback(async () => {
    setMeState('loading');
    try {
      const [profileResp, settingsResp] = await Promise.all([
        bffClient.get<{ user: User }>('/auth/me'),
        bffClient.get<{ settings: SettingsDto }>('/settings'),
      ]);
      setUser(profileResp.user);
      setSettings(settingsResp.settings);
      setMeState('ready');
    } catch (e) {
      setMeState('failed');
      setErrorMsg(e instanceof BffClientError ? e.message : '加载失败');
    }
  }, []);

  useEffect(() => {
    fetchProfile();
  }, [fetchProfile]);

  const handleLogout = async () => {
    track({ event_name: 'locale_setting_save' });
    setAuthToken(null);
    await clearAuthTokens();
    onLogout();
  };

  return (
    <ScreenShell title="我的">
      {meState === 'loading' && (
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={colors.primary} />
        </View>
      )}
      {meState === 'failed' && (
        <View style={styles.centered}>
          <Text style={styles.errorText}>{errorMsg}</Text>
          <TouchableOpacity style={styles.retryButton} onPress={fetchProfile}>
            <Text style={styles.retryButtonText}>重试</Text>
          </TouchableOpacity>
        </View>
      )}
      {meState === 'ready' && (
        <ScrollView style={styles.scrollView}>
          {user && (
            <View style={styles.section}>
              <Text style={styles.nickname}>{user.nickname}</Text>
              <Text style={styles.detail}>{user.locale} / {user.region}</Text>
            </View>
          )}
          {settings && (
            <View style={styles.section}>
              <Text style={styles.sectionTitle}>设置</Text>
              <View style={styles.settingRow}>
                <Text style={styles.settingLabel}>语言</Text>
                <Text style={styles.settingValue}>{settings.language}</Text>
              </View>
              <View style={styles.settingRow}>
                <Text style={styles.settingLabel}>地区</Text>
                <Text style={styles.settingValue}>{settings.region}</Text>
              </View>
              <View style={styles.settingRow}>
                <Text style={styles.settingLabel}>时区</Text>
                <Text style={styles.settingValue}>{settings.timezone}</Text>
              </View>
              <View style={styles.settingRow}>
                <Text style={styles.settingLabel}>通知语言</Text>
                <Text style={styles.settingValue}>{settings.notification_language}</Text>
              </View>
              <View style={styles.settingRow}>
                <Text style={styles.settingLabel}>通知</Text>
                <Text style={styles.settingValue}>{settings.notifications_enabled ? '已开启' : '已关闭'}</Text>
              </View>
            </View>
          )}
          <View style={styles.section}>
            <Text style={styles.sectionTitle}>数据权利</Text>
            {onNavigateSettings && (
              <TouchableOpacity style={styles.linkRow} onPress={onNavigateSettings}>
                <Text style={styles.linkText}>设置</Text>
              </TouchableOpacity>
            )}
            {onNavigateProfile && (
              <TouchableOpacity style={styles.linkRow} onPress={onNavigateProfile}>
                <Text style={styles.linkText}>画像确认</Text>
              </TouchableOpacity>
            )}
            <TouchableOpacity
              style={styles.linkRow}
              onPress={() => track({ event_name: 'data_rights_view' })}
            >
              <Text style={styles.linkText}>导出我的数据</Text>
            </TouchableOpacity>
            <TouchableOpacity
              style={styles.linkRow}
              onPress={() => track({ event_name: 'data_export_request' })}
            >
              <Text style={styles.linkText}>删除我的数据</Text>
            </TouchableOpacity>
            <TouchableOpacity
              style={styles.linkRow}
              onPress={() => track({ event_name: 'data_delete_request' })}
            >
              <Text style={styles.linkText}>纠正我的数据</Text>
            </TouchableOpacity>
            <TouchableOpacity
              style={styles.linkRow}
              onPress={() => track({ event_name: 'data_correction_request' })}
            >
              <Text style={styles.linkText}>查看合规信息</Text>
            </TouchableOpacity>
          </View>
          <TouchableOpacity style={styles.logoutButton} onPress={handleLogout}>
            <Text style={styles.logoutButtonText}>退出登录</Text>
          </TouchableOpacity>
        </ScrollView>
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
  scrollView: {
    flex: 1,
  },
  section: {
    marginBottom: spacing.xl,
  },
  sectionTitle: {
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
    color: colors.textSecondary,
    marginBottom: spacing.sm,
    textTransform: 'uppercase',
  },
  nickname: {
    fontSize: typography.fontSize.xxl,
    fontWeight: typography.fontWeight.bold,
    color: colors.textPrimary,
    marginBottom: spacing.xs,
  },
  detail: {
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
  },
  settingRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  settingLabel: {
    fontSize: typography.fontSize.base,
    color: colors.textPrimary,
  },
  settingValue: {
    fontSize: typography.fontSize.base,
    color: colors.textSecondary,
  },
  linkRow: {
    paddingVertical: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  linkText: {
    fontSize: typography.fontSize.base,
    color: colors.info,
  },
  logoutButton: {
    backgroundColor: colors.errorBg,
    borderRadius: borderRadius.md,
    paddingVertical: spacing.md,
    alignItems: 'center',
    marginTop: spacing.lg,
    marginBottom: spacing.xxxl,
  },
  logoutButtonText: {
    color: colors.error,
    fontSize: typography.fontSize.base,
    fontWeight: typography.fontWeight.semibold,
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
});
