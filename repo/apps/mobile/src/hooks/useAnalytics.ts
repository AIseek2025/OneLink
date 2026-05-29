import { useCallback, useEffect, useRef } from 'react';
import { trackEvent, getAnalyticsContext, AnalyticsEvent, AnalyticsContext } from '../services/analytics';

let globalEndpointConfigured = false;

export function useAnalytics(screenName: string) {
  const contextRef = useRef<AnalyticsContext | null>(null);

  useEffect(() => {
    if (!globalEndpointConfigured) {
      globalEndpointConfigured = true;
    }

    getAnalyticsContext({ screen: screenName }).then((ctx) => {
      contextRef.current = ctx;
      trackEvent({ event_name: 'page.view', page_path: screenName }, ctx);
    });
  }, [screenName]);

  const track = useCallback((event: AnalyticsEvent, overrides?: Partial<AnalyticsContext>) => {
    if (!contextRef.current) {
      getAnalyticsContext({ screen: screenName, ...overrides }).then((ctx) => {
        contextRef.current = ctx;
        trackEvent(event, ctx);
      });
    } else {
      const ctx = { ...contextRef.current, ...overrides };
      contextRef.current = ctx;
      trackEvent(event, ctx);
    }
  }, [screenName]);

  const updateUserId = useCallback((userId: string | null) => {
    if (contextRef.current) {
      contextRef.current = { ...contextRef.current, user_id: userId };
    }
  }, []);

  return { track, updateUserId };
}