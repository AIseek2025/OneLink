import { t, setLocale, getLocale } from '../i18n';

describe('i18n', () => {
  afterEach(() => {
    setLocale('zh-CN');
  });

  it('returns Chinese translations by default', () => {
    expect(t('app.name')).toBe('OneLink');
    expect(t('nav.home')).toBe('首页 / Lumi');
    expect(t('auth.login')).toBe('登录');
    expect(t('state.loading')).toBe('加载中…');
  });

  it('switches to English when locale is set', () => {
    setLocale('en-US');
    expect(t('nav.home')).toBe('Home / Lumi');
    expect(t('auth.login')).toBe('Login');
    expect(t('state.loading')).toBe('Loading…');
  });

  it('returns key when translation is missing', () => {
    expect(t('nonexistent.key')).toBe('nonexistent.key');
  });

  it('getLocale returns current locale', () => {
    expect(getLocale()).toBe('zh-CN');
    setLocale('en-US');
    expect(getLocale()).toBe('en-US');
  });

  it('t accepts explicit locale override', () => {
    expect(t('nav.home', 'en-US')).toBe('Home / Lumi');
    expect(t('nav.home', 'zh-CN')).toBe('首页 / Lumi');
  });

  it('covers all main navigation labels in both locales', () => {
    setLocale('zh-CN');
    expect(t('nav.find')).toBe('找人');
    expect(t('nav.recommendations')).toBe('推荐');
    expect(t('nav.messages')).toBe('消息');
    expect(t('nav.me')).toBe('我的');
    setLocale('en-US');
    expect(t('nav.find')).toBe('Find');
    expect(t('nav.recommendations')).toBe('Recommendations');
    expect(t('nav.messages')).toBe('Messages');
    expect(t('nav.me')).toBe('Me');
  });
});
