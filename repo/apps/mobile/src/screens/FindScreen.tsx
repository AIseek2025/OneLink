import React, { useState } from 'react';
import { View, Text, TextInput, TouchableOpacity, StyleSheet, ActivityIndicator } from 'react-native';
import { ScreenShell } from '../components/ScreenShell';
import { bffClient, BffClientError } from '../services/bffClient';
import { useAnalytics } from '../hooks/useAnalytics';
import { colors, spacing, typography, borderRadius } from '../theme/tokens';
import type { FindRequestCreate } from '../types';

type FindState = 'draft' | 'submitting' | 'submitted' | 'clarification_needed' | 'failed';

export function FindScreen() {
  const { track } = useAnalytics('FindScreen');
  const [intentText, setIntentText] = useState('');
  const [findState, setFindState] = useState<FindState>('draft');
  const [errorMsg, setErrorMsg] = useState('');
  const [requestId, setRequestId] = useState('');

  const handleSubmit = async () => {
    if (!intentText.trim()) return;
    setFindState('submitting');
    setErrorMsg('');
    track({ event_name: 'find.intent.submitted', query: intentText.trim(), query_length: intentText.trim().length });
    try {
      const payload: FindRequestCreate = { intent_text: intentText.trim() };
      const resp = await bffClient.post<{ request_id: string; status: string }>('/find/requests', payload);
      setRequestId(resp.request_id);
      if (resp.status === 'needs_clarification') {
        setFindState('clarification_needed');
      } else {
        setFindState('submitted');
      }
    } catch (e) {
      setFindState('failed');
      const msg = e instanceof BffClientError ? e.message : '提交失败，请重试';
      setErrorMsg(msg);
      track({ event_name: 'error.occurred', error_type: 'runtime', error_message: msg });
    }
  };

  return (
    <ScreenShell title="找人">
      <View style={styles.content}>
        {findState === 'submitted' ? (
          <View style={styles.centered}>
            <Text style={styles.successText}>请求已提交</Text>
            <Text style={styles.detailText}>请求 ID: {requestId}</Text>
            <TouchableOpacity
              style={styles.button}
              onPress={() => { setFindState('draft'); setIntentText(''); setRequestId(''); }}
            >
              <Text style={styles.buttonText}>发起新请求</Text>
            </TouchableOpacity>
          </View>
        ) : findState === 'clarification_needed' ? (
          <View style={styles.centered}>
            <Text style={styles.infoText}>需要补充更多条件</Text>
            <Text style={styles.detailText}>请求 ID: {requestId}</Text>
            <TouchableOpacity
              style={styles.button}
              onPress={() => { setFindState('draft'); setIntentText(''); setRequestId(''); }}
            >
              <Text style={styles.buttonText}>补充条件</Text>
            </TouchableOpacity>
          </View>
        ) : (
          <>
            <TextInput
              style={styles.input}
              placeholder="描述你想找什么样的人…"
              placeholderTextColor={colors.textPlaceholder}
              value={intentText}
              onChangeText={setIntentText}
              multiline
              numberOfLines={4}
              editable={findState !== 'submitting'}
            />
            {errorMsg ? <Text style={styles.errorText}>{errorMsg}</Text> : null}
            <TouchableOpacity
              style={styles.button}
              onPress={handleSubmit}
              disabled={findState === 'submitting' || !intentText.trim()}
            >
              {findState === 'submitting' ? (
                <ActivityIndicator color={colors.chatUserBubbleText} />
              ) : (
                <Text style={styles.buttonText}>提交找人请求</Text>
              )}
            </TouchableOpacity>
          </>
        )}
      </View>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  content: {
    flex: 1,
    gap: spacing.md,
  },
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  input: {
    borderWidth: 1,
    borderColor: colors.border,
    borderRadius: borderRadius.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    fontSize: typography.fontSize.base,
    color: colors.textPrimary,
    minHeight: 100,
    textAlignVertical: 'top',
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
  successText: {
    fontSize: typography.fontSize.lg,
    fontWeight: typography.fontWeight.semibold,
    color: colors.success,
    marginBottom: spacing.sm,
  },
  infoText: {
    fontSize: typography.fontSize.lg,
    fontWeight: typography.fontWeight.semibold,
    color: colors.info,
    marginBottom: spacing.sm,
  },
  detailText: {
    fontSize: typography.fontSize.sm,
    color: colors.textSecondary,
    marginBottom: spacing.lg,
  },
});
