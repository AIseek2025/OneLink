import { afterAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { LoginPage } from './pages/LoginPage';
import { RecommendationsPage } from './pages/RecommendationsPage';
import { DmPage } from './pages/DmPage';
import { SettingsPage } from './pages/SettingsPage';

vi.mock('./analytics', () => ({
  trackEvent: vi.fn(),
  getAnalyticsContext: vi.fn(() => ({ screen: 'smoke', user_id: 'user-1' })),
}));

type MockResponse = {
  status: number;
  body?: unknown;
};

const originalLocation = window.location;

function jsonResponse({ status, body }: MockResponse): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
    text: async () => JSON.stringify(body ?? {}),
  } as Response;
}

function installLocationStub(initialHref = 'http://localhost/login') {
  let href = initialHref;
  Object.defineProperty(window, 'location', {
    configurable: true,
    value: {
      get href() {
        return href;
      },
      set href(value: string) {
        href = value;
      },
      assign(value: string) {
        href = value;
      },
      replace(value: string) {
        href = value;
      },
      reload() {},
    },
  });
}

function installFetchMock(
  handler: (url: string, init?: RequestInit) => MockResponse | Promise<MockResponse>,
) {
  const requests: Array<{ url: string; method: string; body?: string }> = [];
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === 'string'
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    requests.push({
      url,
      method: init?.method ?? 'GET',
      body: typeof init?.body === 'string' ? init.body : undefined,
    });
    return jsonResponse(await handler(url, init));
  });

  vi.stubGlobal('fetch', fetchMock);
  return { fetchMock, requests };
}

describe('pre-launch web smoke', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
    installLocationStub();
  });

  afterAll(() => {
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: originalLocation,
    });
  });

  it('covers login happy path', async () => {
    const { requests } = installFetchMock(async (url) => {
      if (url.endsWith('/api/v1/bff/auth/login')) {
        return {
          status: 200,
          body: {
            user: { user_id: 'user-1' },
            session: { token: 'token-1', expires_at: '2026-06-28T00:00:00Z' },
          },
        };
      }
      throw new Error(`unhandled request: ${url}`);
    });

    render(<LoginPage />);

    fireEvent.change(screen.getByPlaceholderText('邮箱'), {
      target: { value: 'smoke@example.com' },
    });
    fireEvent.change(screen.getByPlaceholderText('密码'), {
      target: { value: 'smoke-pass' },
    });
    fireEvent.click(screen.getByRole('button', { name: '登录' }));

    await waitFor(() => {
      expect(localStorage.getItem('onelink_token')).toBe('token-1');
      expect(window.location.href).toBe('/');
    });

    expect(requests).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          url: '/api/v1/bff/auth/login',
          method: 'POST',
        }),
      ]),
    );
  });

  it('covers recommendation detail, feedback and connect flow', async () => {
    localStorage.setItem('onelink_token', 'token-1');
    const { requests } = installFetchMock(async (url, init) => {
      if (url.endsWith('/api/v1/bff/recommendations') && !init?.method) {
        return {
          status: 200,
          body: {
            recommendations: [
              {
                recommendation_id: 'rec-1',
                user_id: 'user-2',
                display_name: '测试用户',
                headline: 'AI infra founder',
                city_level_location: 'Shanghai',
                match_score: 0.92,
                interest_tags: ['AI', 'Infra'],
              },
            ],
          },
        };
      }
      if (url.endsWith('/api/v1/bff/recommendations/rec-1')) {
        return {
          status: 200,
          body: {
            recommendation_id: 'rec-1',
            user_id: 'user-2',
            display_name: '测试用户',
            headline: 'AI infra founder',
            city_level_location: 'Shanghai',
            match_score: 0.92,
            interest_tags: ['AI', 'Infra'],
            explanation: '共同关注 AI 基础设施',
            profile_summary: '长期关注 AI infra 与开源生态',
            shared_interests: ['AI', 'Open Source'],
          },
        };
      }
      if (url.endsWith('/api/v1/bff/recommendations/rec-1/feedback')) {
        return { status: 200, body: { status: 'ok' } };
      }
      if (url.endsWith('/api/v1/bff/dm/threads/draft')) {
        return { status: 200, body: { thread_id: 'thread-1' } };
      }
      if (url.endsWith('/api/v1/bff/dm/threads/first-message')) {
        return { status: 200, body: { message_id: 'msg-1' } };
      }
      throw new Error(`unhandled request: ${url}`);
    });

    render(<RecommendationsPage />);

    await screen.findByText('测试用户');
    fireEvent.click(screen.getByText('测试用户'));
    await screen.findByText('匹配解释');

    fireEvent.click(screen.getByRole('button', { name: '反馈' }));
    fireEvent.click(screen.getByRole('button', { name: '满意' }));
    fireEvent.change(screen.getByPlaceholderText('补充说明（可选）'), {
      target: { value: '推荐结果和画像一致' },
    });
    fireEvent.click(screen.getByRole('button', { name: '提交反馈' }));
    await screen.findByText('反馈已提交');

    fireEvent.click(screen.getByRole('button', { name: '← 返回详情' }));
    fireEvent.click(screen.getByRole('button', { name: '发起私信' }));
    fireEvent.change(screen.getByPlaceholderText('写一段开场白...'), {
      target: { value: '你好，想交流一下 AI Infra 经验。' },
    });
    fireEvent.click(screen.getByRole('button', { name: '发送私信' }));
    await screen.findByText('私信已发送');

    expect(requests).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ url: '/api/v1/bff/recommendations', method: 'GET' }),
        expect.objectContaining({ url: '/api/v1/bff/recommendations/rec-1', method: 'GET' }),
        expect.objectContaining({ url: '/api/v1/bff/recommendations/rec-1/feedback', method: 'POST' }),
        expect.objectContaining({ url: '/api/v1/bff/dm/threads/draft', method: 'POST' }),
        expect.objectContaining({ url: '/api/v1/bff/dm/threads/first-message', method: 'POST' }),
      ]),
    );
  });

  it('covers dm thread loading, safety notice and first message send', async () => {
    localStorage.setItem('onelink_token', 'token-1');
    const { requests } = installFetchMock(async (url, init) => {
      if (url.endsWith('/api/v1/bff/conversations')) {
        return {
          status: 200,
          body: {
            threads: [
              {
                thread_id: 'thread-1',
                other_user_id: 'user-2',
                other_user_name: '候选人 A',
                last_message_preview: 'hello',
                last_message_at: '2026-05-28T00:00:00Z',
                state: 'active',
              },
            ],
          },
        };
      }
      if (url.endsWith('/api/v1/bff/dm/threads/thread-1')) {
        return {
          status: 200,
          body: {
            messages: [
              {
                id: 'msg-0',
                sender_id: 'user-2',
                content: '你好，欢迎来聊',
                created_at: '2026-05-28T00:00:00Z',
              },
            ],
            safety_notice: '首条私信正在安全审核中，发送可能受限',
          },
        };
      }
      if (url.endsWith('/api/v1/bff/dm/threads/first-message') && init?.method === 'POST') {
        return { status: 200, body: { message_id: 'msg-1' } };
      }
      throw new Error(`unhandled request: ${url}`);
    });

    render(<DmPage />);

    await screen.findByText('候选人 A');
    fireEvent.click(screen.getByText('候选人 A'));
    await screen.findByText('首条私信正在安全审核中，发送可能受限');
    await screen.findByText('你好，欢迎来聊');

    fireEvent.change(screen.getByPlaceholderText('输入消息...'), {
      target: { value: '收到，继续沟通。' },
    });
    fireEvent.click(screen.getByRole('button', { name: '发送' }));

    await waitFor(() => {
      expect(
        requests.some(
          (request) =>
            request.url === '/api/v1/bff/dm/threads/first-message' &&
            request.method === 'POST',
        ),
      ).toBe(true);
    });
  });

  it('covers settings load and save', async () => {
    localStorage.setItem('onelink_token', 'token-1');
    const { requests } = installFetchMock(async (url, init) => {
      if (url.endsWith('/api/v1/bff/users/me') && !init?.method) {
        return {
          status: 200,
          body: {
            locale: 'zh-CN',
            primary_region: 'CN',
            timezone: 'Asia/Shanghai',
            notification_language: 'zh-CN',
            allow_search: true,
            allow_recommend: true,
          },
        };
      }
      if (url.endsWith('/api/v1/bff/settings/locale') && init?.method === 'PATCH') {
        return { status: 200, body: { status: 'ok' } };
      }
      if (url.endsWith('/api/v1/bff/users/me') && init?.method === 'PATCH') {
        return { status: 200, body: { status: 'ok' } };
      }
      throw new Error(`unhandled request: ${url}`);
    });

    render(<SettingsPage />);

    const localeSelect = (await screen.findAllByRole('combobox'))[0];
    fireEvent.change(localeSelect, { target: { value: 'en-US' } });
    fireEvent.blur(localeSelect);
    await screen.findByText('设置已保存');

    const toggles = screen.getAllByText(/允许被/);
    fireEvent.click(toggles[0].parentElement?.parentElement?.lastElementChild as Element);
    await screen.findByText('设置已保存');

    expect(requests).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ url: '/api/v1/bff/users/me', method: 'GET' }),
        expect.objectContaining({ url: '/api/v1/bff/settings/locale', method: 'PATCH' }),
        expect.objectContaining({ url: '/api/v1/bff/users/me', method: 'PATCH' }),
      ]),
    );
  });

  it('covers exception state and retry on recommendations page', async () => {
    localStorage.setItem('onelink_token', 'token-1');
    let recommendationCallCount = 0;
    installFetchMock(async (url) => {
      if (url.endsWith('/api/v1/bff/recommendations')) {
        recommendationCallCount += 1;
        if (recommendationCallCount === 1) {
          return { status: 500, body: { error: 'boom' } };
        }
        return { status: 200, body: { recommendations: [] } };
      }
      throw new Error(`unhandled request: ${url}`);
    });

    render(<RecommendationsPage />);

    await screen.findByText('recommendations failed: 500');
    fireEvent.click(screen.getByRole('button', { name: '重试' }));
    await screen.findByText('暂无推荐');
  });
});
