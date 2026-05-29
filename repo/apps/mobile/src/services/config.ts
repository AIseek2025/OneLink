import AsyncStorage from '@react-native-async-storage/async-storage';

export const BFF_BASE_URL = __DEV__
  ? 'http://10.0.2.2:3000'
  : 'https://api.onelink.app';

export const API_PREFIX = '/api/v1/bff';

export function bffUrl(path: string): string {
  return `${BFF_BASE_URL}${API_PREFIX}${path}`;
}

const AUTH_TOKEN_KEY = 'onelink_auth_token';
const REFRESH_TOKEN_KEY = 'onelink_refresh_token';
const LOCALE_KEY = 'onelink_locale';
const REGION_KEY = 'onelink_region';

export async function persistAuthTokens(access: string, refresh?: string): Promise<void> {
  await AsyncStorage.setItem(AUTH_TOKEN_KEY, access);
  if (refresh) {
    await AsyncStorage.setItem(REFRESH_TOKEN_KEY, refresh);
  }
}

export async function getPersistedAccessToken(): Promise<string | null> {
  return AsyncStorage.getItem(AUTH_TOKEN_KEY);
}

export async function getPersistedRefreshToken(): Promise<string | null> {
  return AsyncStorage.getItem(REFRESH_TOKEN_KEY);
}

export async function clearAuthTokens(): Promise<void> {
  await AsyncStorage.multiRemove([AUTH_TOKEN_KEY, REFRESH_TOKEN_KEY]);
}

export async function getPersistedLocale(): Promise<string> {
  return (await AsyncStorage.getItem(LOCALE_KEY)) || 'zh-CN';
}

export async function setPersistedLocale(locale: string): Promise<void> {
  await AsyncStorage.setItem(LOCALE_KEY, locale);
}

export async function getPersistedRegion(): Promise<string> {
  return (await AsyncStorage.getItem(REGION_KEY)) || 'CN';
}

export async function setPersistedRegion(region: string): Promise<void> {
  await AsyncStorage.setItem(REGION_KEY, region);
}
