import React, { useEffect, useState } from 'react';
import { View, Text, StyleSheet, ActivityIndicator } from 'react-native';
import { bffClient, setAuthToken } from '../services/bffClient';
import { getPersistedAccessToken } from '../services/config';
import { trackScreenSpecEvent } from '../services/tracking';
import { colors, spacing, typography } from '../theme/tokens';

type BootState = 'booting' | 'restoring_session' | 'session_restored' | 'no_session' | 'boot_failed';

interface SplashScreenProps {
  onBootComplete: (hasSession: boolean) => void;
}

export function SplashScreen({ onBootComplete }: SplashScreenProps) {
  const [bootState, setBootState] = useState<BootState>('booting');

  useEffect(() => {
    const boot = async () => {
      trackScreenSpecEvent('app_boot_started');
      setBootState('restoring_session');
      try {
        const token = await getPersistedAccessToken();
        if (token) {
          setAuthToken(token);
          try {
            await bffClient.get('/auth/me');
            setBootState('session_restored');
            trackScreenSpecEvent('app_boot_finished');
            onBootComplete(true);
            return;
          } catch {
            setAuthToken(null);
          }
        }
        setBootState('no_session');
        trackScreenSpecEvent('app_boot_finished');
        onBootComplete(false);
      } catch {
        setBootState('boot_failed');
        trackScreenSpecEvent('app_boot_failed');
        onBootComplete(false);
      }
    };
    boot();
  }, [onBootComplete]);

  return (
    <View style={styles.container}>
      <ActivityIndicator size="large" color={colors.primary} />
      <Text style={styles.statusText}>
        {bootState === 'booting' || bootState === 'restoring_session'
          ? '正在启动…'
          : bootState === 'boot_failed'
          ? '启动失败，请重试'
          : ''}
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    backgroundColor: colors.background,
  },
  statusText: {
    marginTop: spacing.lg,
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
  },
});
