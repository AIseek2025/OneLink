import React, { useState } from 'react';
import { View, Text, TextInput, TouchableOpacity, StyleSheet, ActivityIndicator, ScrollView } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, setAuthToken, BffClientError } from '../services/bffClient';
import { persistAuthTokens } from '../services/config';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';
import type { AuthResponse } from '../types';

type RegisterState = 'idle' | 'validating' | 'submitting' | 'registered' | 'requires_verification' | 'failed';

export function RegisterScreen({ onRegisterSuccess, onSwitchToLogin }: { onRegisterSuccess: () => void; onSwitchToLogin?: () => void }) {
  const { track, updateUserId } = useAnalytics('RegisterScreen');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [nickname, setNickname] = useState('');
  const [registerState, setRegisterState] = useState<RegisterState>('idle');
  const [errorMsg, setErrorMsg] = useState('');

  const handleRegister = async () => {
    setRegisterState('submitting');
    setErrorMsg('');
    track({ event_name: 'registration.started', provider: 'email' });
    try {
      const resp = await bffClient.post<AuthResponse>('/auth/register', {
        email,
        password,
        nickname,
      });
      setAuthToken(resp.access_token);
      await persistAuthTokens(resp.access_token, resp.refresh_token);
      updateUserId(resp.user_id);
      track({ event_name: 'registration.completed', user_id: resp.user_id, provider: 'email' });
      if (resp.flow_state === 'requires_verification') {
        setRegisterState('requires_verification');
      } else {
        setRegisterState('registered');
        onRegisterSuccess();
      }
    } catch (e) {
      setRegisterState('failed');
      const msg = e instanceof BffClientError ? e.message : '注册失败，请重试';
      setErrorMsg(msg);
      track({ event_name: 'error.occurred', error_type: 'runtime', error_message: msg });
    }
  };

  return (
    <ScreenShell title="注册">
      <ScrollView style={styles.scrollView}>
        <View style={styles.form}>
          <TextInput
            style={styles.input}
            placeholder="邮箱"
            placeholderTextColor={colors.textPlaceholder}
            value={email}
            onChangeText={setEmail}
            autoCapitalize="none"
            keyboardType="email-address"
            editable={registerState !== 'submitting'}
          />
          <TextInput
            style={styles.input}
            placeholder="昵称"
            placeholderTextColor={colors.textPlaceholder}
            value={nickname}
            onChangeText={setNickname}
            editable={registerState !== 'submitting'}
          />
          <TextInput
            style={styles.input}
            placeholder="密码"
            placeholderTextColor={colors.textPlaceholder}
            value={password}
            onChangeText={setPassword}
            secureTextEntry
            editable={registerState !== 'submitting'}
          />
          {errorMsg ? <Text style={styles.errorText}>{errorMsg}</Text> : null}
          {registerState === 'requires_verification' ? (
            <Text style={styles.infoText}>请查看邮箱完成验证</Text>
          ) : null}
          <TouchableOpacity
            style={styles.button}
            onPress={handleRegister}
            disabled={registerState === 'submitting'}
          >
            {registerState === 'submitting' ? (
              <ActivityIndicator color={colors.chatUserBubbleText} />
            ) : (
              <Text style={styles.buttonText}>注册</Text>
            )}
          </TouchableOpacity>
          {onSwitchToLogin ? (
            <TouchableOpacity onPress={onSwitchToLogin} style={styles.linkButton}>
              <Text style={styles.linkText}>已有账号？去登录</Text>
            </TouchableOpacity>
          ) : null}
        </View>
      </ScrollView>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  scrollView: {
    flex: 1,
  },
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
  infoText: {
    color: colors.info,
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
