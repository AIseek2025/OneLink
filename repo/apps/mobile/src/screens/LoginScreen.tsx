import React, { useEffect, useState, useCallback } from 'react';
import { View, Text, TextInput, TouchableOpacity, StyleSheet, ActivityIndicator } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, setAuthToken, BffClientError } from '../services/bffClient';
import { persistAuthTokens, clearAuthTokens, getPersistedAccessToken } from '../services/config';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';
import type { AuthResponse } from '../types';

type LoginState = 'idle' | 'submitting' | 'success' | 'failed' | 'expired_session';

export function LoginScreen({ onLoginSuccess, onSwitchToRegister }: { onLoginSuccess: () => void; onSwitchToRegister?: () => void }) {
  const { track, updateUserId } = useAnalytics('LoginScreen');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [loginState, setLoginState] = useState<LoginState>('idle');
  const [errorMsg, setErrorMsg] = useState('');

  const tryRestoreSession = useCallback(async () => {
    const token = await getPersistedAccessToken();
    if (token) {
      setAuthToken(token);
      setLoginState('success');
      onLoginSuccess();
    }
  }, [onLoginSuccess]);

  useEffect(() => {
    tryRestoreSession();
  }, [tryRestoreSession]);

  const handleLogin = async () => {
    setLoginState('submitting');
    setErrorMsg('');
    track({ event_name: 'login.started', provider: 'email' });
    try {
      const resp = await bffClient.post<AuthResponse>('/auth/login', { email, password });
      setAuthToken(resp.access_token);
      await persistAuthTokens(resp.access_token, resp.refresh_token);
      updateUserId(resp.user_id);
      track({ event_name: 'login.completed', user_id: resp.user_id, provider: 'email' });
      setLoginState('success');
      onLoginSuccess();
    } catch (e) {
      if (e instanceof BffClientError && e.status === 401) {
        await clearAuthTokens();
        setLoginState('expired_session');
        setErrorMsg('会话已过期，请重新登录');
        track({ event_name: 'error.occurred', error_type: 'auth', error_code: 'E401' });
      } else {
        setLoginState('failed');
        const msg = e instanceof Error ? e.message : '登录失败，请重试';
        setErrorMsg(msg);
        track({ event_name: 'error.occurred', error_type: 'runtime', error_message: msg });
      }
    }
  };

  return (
    <ScreenShell title="登录">
      <View style={styles.form}>
        <TextInput
          style={styles.input}
          placeholder="邮箱"
          placeholderTextColor={colors.textPlaceholder}
          value={email}
          onChangeText={setEmail}
          autoCapitalize="none"
          keyboardType="email-address"
          editable={loginState !== 'submitting'}
        />
        <TextInput
          style={styles.input}
          placeholder="密码"
          placeholderTextColor={colors.textPlaceholder}
          value={password}
          onChangeText={setPassword}
          secureTextEntry
          editable={loginState !== 'submitting'}
        />
        {errorMsg ? <Text style={styles.errorText}>{errorMsg}</Text> : null}
        <TouchableOpacity
          style={styles.button}
          onPress={handleLogin}
          disabled={loginState === 'submitting'}
        >
          {loginState === 'submitting' ? (
            <ActivityIndicator color={colors.chatUserBubbleText} />
          ) : (
            <Text style={styles.buttonText}>登录</Text>
          )}
        </TouchableOpacity>
        {onSwitchToRegister ? (
          <TouchableOpacity onPress={onSwitchToRegister} style={styles.linkButton}>
            <Text style={styles.linkText}>没有账号？去注册</Text>
          </TouchableOpacity>
        ) : null}
      </View>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  form: {
    gap: spacing.md,
  },
  input: {
    borderWidth: 1,
    borderColor: colors.border,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    fontSize: typography.fontSize.base,
    color: colors.textPrimary,
  },
  button: {
    backgroundColor: colors.primary,
    borderRadius: borderRadius.md,
    paddingVertical: spacing.md,
    alignItems: 'center',
  },
  buttonText: {
    color: colors.chatUserBubbleText,
    fontSize: typography.fontSize.base,
    fontWeight: typography.fontWeight.semibold,
  },
  errorText: {
    color: colors.error,
    fontSize: typography.fontSize.sm,
  },
  linkButton: {
    alignItems: 'center',
    paddingVertical: spacing.sm,
  },
  linkText: {
    color: colors.primary,
    fontSize: typography.fontSize.sm,
  },
});
