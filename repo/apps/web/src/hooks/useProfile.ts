import { useState, useCallback, useEffect } from 'react';
import { fetchProfile, patchProfile } from '../api/client';
import { trackEvent, getAnalyticsContext } from '../analytics';
import { getStoredUserId } from '../auth/session';

interface ProfileData {
  user: { user_id: string; status: string; primary_region: string };
  profile: {
    display_name: string;
    avatar_url: string;
    city_level_location: string;
    languages: string[];
    is_searchable: boolean;
    allow_discovery: boolean;
    facts: Array<{ fact_type: string; value: string; confidence: number }>;
    traits: {
      interest_tags: string[];
      connection_goal_tags: string[];
      location_label: string | null;
      communication_preferences: string[];
    };
  } | null;
  completion: { completion_rate: number; missing_dimensions: string[] } | null;
}

export function useProfile(userId?: string) {
  const [data, setData] = useState<ProfileData | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const uid = userId ?? getStoredUserId() ?? 'me';
      const d = await fetchProfile(uid);
      setData(d);
      const uid2 = d.user?.user_id ?? null;
      trackEvent({ event_name: 'settings_view' }, getAnalyticsContext({ screen: '/profile', user_id: uid2 }));
      if (d.completion) {
        trackEvent(
          { event_name: 'profile_fact_exposed', completion_rate: d.completion.completion_rate, missing_dimensions: d.completion.missing_dimensions },
          getAnalyticsContext({ screen: '/profile', user_id: uid2 }),
        );
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to load profile';
      setError(msg);
      trackEvent({ event_name: 'error.occurred', error_type: 'network', error_message: msg }, getAnalyticsContext({ screen: '/profile' }));
    } finally {
      setIsLoading(false);
    }
  }, [userId]);

  useEffect(() => { load(); }, [load]);

  const save = useCallback(async (fields: Record<string, unknown>) => {
    if (!data || Object.keys(fields).length === 0) return false;
    setIsSaving(true);
    setError(null);
    try {
      await patchProfile(fields);
      trackEvent(
        { event_name: 'profile.fact.confirmed', fact_type: 'profile_edit', fact_value: JSON.stringify(fields) },
        getAnalyticsContext({ screen: '/profile', user_id: data.user?.user_id ?? null }),
      );
      await load();
      return true;
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Save failed';
      setError(msg);
      trackEvent({ event_name: 'error.occurred', error_type: 'network', error_message: msg }, getAnalyticsContext({ screen: '/profile' }));
      return false;
    } finally {
      setIsSaving(false);
    }
  }, [data, load]);

  const clearError = useCallback(() => setError(null), []);

  return {
    data,
    isLoading,
    isSaving,
    error,
    save,
    reload: load,
    clearError,
  };
}
