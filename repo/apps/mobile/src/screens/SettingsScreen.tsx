import React, { useEffect, useState, useCallback } from 'react';
import { View, Text, ScrollView, TouchableOpacity, StyleSheet, ActivityIndicator } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, BffClientError } from '../services/bffClient';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';

const LOCALE_OPTIONS = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'zh-TW', label: '繁體中文' },
  { value: 'en-US', label: 'English (US)' },
  { value: 'en-GB', label: 'English (UK)' },
  { value: 'ja-JP', label: '日本語' },
  { value: 'ko-KR', label: '한국어' },
];

const REGION_OPTIONS = [
  { value: 'CN', label: '中国大陆' },
  { value: 'HK', label: '香港' },
  { value: 'TW', label: '台湾' },
  { value: 'SG', label: '新加坡' },
  { value: 'JP', label: '日本' },
  { value: 'KR', label: '韩国' },
  { value: 'US', label: '美国' },
  { value: 'GB', label: '英国' },
];

const TIMEZONE_OPTIONS = [
  { value: 'Asia/Shanghai', label: 'Asia/Shanghai (UTC+8)' },
  { value: 'Asia/Hong_Kong', label: 'Asia/Hong_Kong (UTC+8)' },
  { value: 'Asia/Taipei', label: 'Asia/Taipei (UTC+8)' },
  { value: 'Asia/Tokyo', label: 'Asia/Tokyo (UTC+9)' },
  { value: 'Asia/Seoul', label: 'Asia/Seoul (UTC+9)' },
  { value: 'Asia/Singapore', label: 'Asia/Singapore (UTC+8)' },
  { value: 'America/New_York', label: 'America/New_York (UTC-5)' },
  { value: 'America/Los_Angeles', label: 'America/Los_Angeles (UTC-8)' },
  { value: 'Europe/London', label: 'Europe/London (UTC+0)' },
];

const NOTIFICATION_LANG_OPTIONS = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'zh-TW', label: '繁體中文' },
  { value: 'en-US', label: 'English' },
  { value: 'ja-JP', label: '日本語' },
  { value: 'ko-KR', label: '한국어' },
];

type SettingsState = 'loading' | 'ready' | 'failed';

export function SettingsScreen({ onNavigateProfile, onNavigateCompliance }: { onNavigateProfile?: () => void; onNavigateCompliance?: () => void }) {
  const { track } = useAnalytics('SettingsScreen');
  const [settingsState, setSettingsState] = useState<SettingsState>('loading');
  const [locale, setLocale] = useState('zh-CN');
  const [region, setRegion] = useState('CN');
  const [timezone, setTimezone] = useState('Asia/Shanghai');
  const [notifLang, setNotifLang] = useState('zh-CN');
  const [allowSearch, setAllowSearch] = useState(true);
  const [allowRecommend, setAllowRecommend] = useState(true);
  const [errorMsg, setErrorMsg] = useState('');
  const [successMsg, setSuccessMsg] = useState('');

  const loadSettings = useCallback(async () => {
    setSettingsState('loading');
    try {
      const resp = await bffClient.get<{ settings: { locale?: string; region?: string; timezone?: string; notification_language?: string; allow_search?: boolean; allow_recommend?: boolean } }>('/settings');
      const s = resp.settings;
      if (s.locale) setLocale(s.locale);
      if (s.region) setRegion(s.region);
      if (s.timezone) setTimezone(s.timezone);
      if (s.notification_language) setNotifLang(s.notification_language);
      if (s.allow_search !== undefined) setAllowSearch(s.allow_search);
      if (s.allow_recommend !== undefined) setAllowRecommend(s.allow_recommend);
      setSettingsState('ready');
    } catch (e) {
      setSettingsState('failed');
      setErrorMsg(e instanceof BffClientError ? e.message : '加载失败');
    }
  }, []);

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  const saveLocale = async (field: Record<string, string>) => {
    setErrorMsg('');
    setSuccessMsg('');
    try {
      await bffClient.patch('/settings/locale', field);
      setSuccessMsg('已保存');
      track({ event_name: 'locale_setting_save' });
      setTimeout(() => setSuccessMsg(''), 2000);
    } catch (e) {
      setErrorMsg(e instanceof BffClientError ? e.message : '保存失败');
    }
  };

  const savePrivacy = async (field: Record<string, unknown>) => {
    setErrorMsg('');
    setSuccessMsg('');
    try {
      await bffClient.patch('/users/me', field);
      setSuccessMsg('已保存');
      track({ event_name: 'settings.saved' });
      setTimeout(() => setSuccessMsg(''), 2000);
    } catch (e) {
      setErrorMsg(e instanceof BffClientError ? e.message : '保存失败');
    }
  };

  const handleNavigateCompliance = () => {
    onNavigateCompliance?.();
  };

  if (settingsState === 'loading') {
    return (
      <ScreenShell title="设置">
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={colors.primary} />
        </View>
      </ScreenShell>
    );
  }

  if (settingsState === 'failed') {
    return (
      <ScreenShell title="设置">
        <View style={styles.centered}>
          <Text style={styles.errorText}>{errorMsg}</Text>
          <TouchableOpacity style={styles.retryButton} onPress={loadSettings}>
            <Text style={styles.retryButtonText}>重试</Text>
          </TouchableOpacity>
        </View>
      </ScreenShell>
    );
  }

  return (
    <ScreenShell title="设置">
      <ScrollView>
        {errorMsg ? <View style={styles.errorBanner}><Text style={styles.errorBannerText}>{errorMsg}</Text></View> : null}
        {successMsg ? <View style={styles.successBanner}><Text style={styles.successBannerText}>{successMsg}</Text></View> : null}

        <Text style={styles.sectionTitle}>区域与语言</Text>
        <View style={styles.fieldRow}>
          <Text style={styles.fieldLabel}>界面语言</Text>
          <View style={styles.optionsRow}>
            {LOCALE_OPTIONS.map((opt) => (
              <TouchableOpacity
                key={opt.value}
                style={[styles.optionChip, locale === opt.value ? styles.optionChipActive : null]}
                onPress={() => { setLocale(opt.value); saveLocale({ locale: opt.value }); }}
              >
                <Text style={[styles.optionChipText, locale === opt.value ? styles.optionChipTextActive : null]}>{opt.label}</Text>
              </TouchableOpacity>
            ))}
          </View>
        </View>

        <View style={styles.fieldRow}>
          <Text style={styles.fieldLabel}>地区</Text>
          <View style={styles.optionsRow}>
            {REGION_OPTIONS.map((opt) => (
              <TouchableOpacity
                key={opt.value}
                style={[styles.optionChip, region === opt.value ? styles.optionChipActive : null]}
                onPress={() => { setRegion(opt.value); saveLocale({ region: opt.value }); }}
              >
                <Text style={[styles.optionChipText, region === opt.value ? styles.optionChipTextActive : null]}>{opt.label}</Text>
              </TouchableOpacity>
            ))}
          </View>
        </View>

        <View style={styles.fieldRow}>
          <Text style={styles.fieldLabel}>时区</Text>
          <View style={styles.optionsRow}>
            {TIMEZONE_OPTIONS.map((opt) => (
              <TouchableOpacity
                key={opt.value}
                style={[styles.optionChip, timezone === opt.value ? styles.optionChipActive : null]}
                onPress={() => { setTimezone(opt.value); saveLocale({ timezone: opt.value }); }}
              >
                <Text style={[styles.optionChipText, timezone === opt.value ? styles.optionChipTextActive : null]}>{opt.label}</Text>
              </TouchableOpacity>
            ))}
          </View>
        </View>

        <View style={styles.fieldRow}>
          <Text style={styles.fieldLabel}>通知语言</Text>
          <View style={styles.optionsRow}>
            {NOTIFICATION_LANG_OPTIONS.map((opt) => (
              <TouchableOpacity
                key={opt.value}
                style={[styles.optionChip, notifLang === opt.value ? styles.optionChipActive : null]}
                onPress={() => { setNotifLang(opt.value); saveLocale({ notification_language: opt.value }); }}
              >
                <Text style={[styles.optionChipText, notifLang === opt.value ? styles.optionChipTextActive : null]}>{opt.label}</Text>
              </TouchableOpacity>
            ))}
          </View>
        </View>

        <Text style={styles.sectionTitle}>隐私</Text>
        <View style={styles.toggleRow}>
          <View>
            <Text style={styles.fieldLabel}>允许被搜索</Text>
            <Text style={styles.fieldHint}>其他用户可以通过条件搜索到您</Text>
          </View>
          <TouchableOpacity
            style={[styles.toggle, allowSearch ? styles.toggleOn : null]}
            onPress={() => { const v = !allowSearch; setAllowSearch(v); savePrivacy({ allow_search: v }); }}
          >
            <View style={[styles.toggleKnob, allowSearch ? styles.toggleKnobOn : null]} />
          </TouchableOpacity>
        </View>
        <View style={styles.toggleRow}>
          <View>
            <Text style={styles.fieldLabel}>允许被推荐</Text>
            <Text style={styles.fieldHint}>系统会将您推荐给其他用户</Text>
          </View>
          <TouchableOpacity
            style={[styles.toggle, allowRecommend ? styles.toggleOn : null]}
            onPress={() => { const v = !allowRecommend; setAllowRecommend(v); savePrivacy({ allow_recommend: v }); }}
          >
            <View style={[styles.toggleKnob, allowRecommend ? styles.toggleKnobOn : null]} />
          </TouchableOpacity>
        </View>

        <Text style={styles.sectionTitle}>数据与安全</Text>
        {onNavigateCompliance && (
          <TouchableOpacity style={styles.linkRow} onPress={handleNavigateCompliance}>
            <Text style={styles.linkText}>数据合规中心</Text>
          </TouchableOpacity>
        )}
        {onNavigateProfile && (
          <TouchableOpacity style={styles.linkRow} onPress={onNavigateProfile}>
            <Text style={styles.linkText}>画像确认</Text>
          </TouchableOpacity>
        )}

        <Text style={styles.sectionTitle}>关于</Text>
        <View style={styles.versionRow}>
          <Text style={styles.fieldLabel}>版本</Text>
          <Text style={styles.versionText}>0.3.0</Text>
        </View>
      </ScrollView>
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
  sectionTitle: {
    fontSize: typography.fontSize.sm,
    fontWeight: typography.fontWeight.semibold,
    color: colors.textSecondary,
    textTransform: 'uppercase',
    marginTop: spacing.xl,
    marginBottom: spacing.md,
  },
  fieldRow: {
    marginBottom: spacing.md,
  },
  fieldLabel: {
    fontSize: typography.fontSize.base,
    fontWeight: typography.fontWeight.medium,
    color: colors.textPrimary,
    marginBottom: spacing.xs,
  },
  fieldHint: {
    fontSize: typography.fontSize.xs,
    color: colors.textSecondary,
  },
  optionsRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: spacing.xs,
  },
  optionChip: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.md,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.background,
  },
  optionChipActive: {
    backgroundColor: colors.primaryLight,
    borderColor: colors.primary,
  },
  optionChipText: {
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
  },
  optionChipTextActive: {
    color: colors.primary,
    fontWeight: typography.fontWeight.semibold,
  },
  toggleRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  toggle: {
    width: 44,
    height: 24,
    borderRadius: 12,
    backgroundColor: colors.border,
    justifyContent: 'center',
  },
  toggleOn: {
    backgroundColor: colors.primary,
  },
  toggleKnob: {
    width: 20,
    height: 20,
    borderRadius: 10,
    backgroundColor: '#FFFFFF',
    marginLeft: 2,
  },
  toggleKnobOn: {
    marginLeft: 22,
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
  versionRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingVertical: spacing.sm,
  },
  versionText: {
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
  },
});
