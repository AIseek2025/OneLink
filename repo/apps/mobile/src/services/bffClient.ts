import { bffUrl } from './config';

let authToken: string | null = null;

export function setAuthToken(token: string | null) {
  authToken = token;
}

export function getAuthToken(): string | null {
  return authToken;
}

export type BffError = {
  status: number;
  code: string;
  message: string;
  trace_id?: string;
};

export class BffClientError extends Error {
  public status: number;
  public code: string;
  public traceId?: string;

  constructor(bffError: BffError) {
    super(bffError.message);
    this.name = 'BffClientError';
    this.status = bffError.status;
    this.code = bffError.code;
    this.traceId = bffError.trace_id;
  }
}

async function request<T>(
  path: string,
  options: {
    method?: string;
    body?: unknown;
    locale?: string;
  } = {},
): Promise<T> {
  const { method = 'GET', body, locale = 'zh-CN' } = options;
  const headers: Record<string, string> = {
    'Accept-Language': locale,
    'Content-Type': 'application/json',
  };
  if (authToken) {
    headers['Authorization'] = `Bearer ${authToken}`;
  }
  const resp = await fetch(bffUrl(path), {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!resp.ok) {
    let bffError: BffError;
    try {
      const parsed = await resp.json();
      bffError = {
        status: resp.status,
        code: parsed.code || 'UNKNOWN',
        message: parsed.message || `BFF ${path} failed: ${resp.status}`,
        trace_id: parsed.trace_id,
      };
    } catch {
      bffError = {
        status: resp.status,
        code: 'NETWORK_ERROR',
        message: `BFF ${path} failed: ${resp.status}`,
      };
    }
    throw new BffClientError(bffError);
  }
  return resp.json() as Promise<T>;
}

export const bffClient = {
  get: <T>(path: string, locale?: string) =>
    request<T>(path, { method: 'GET', locale }),

  post: <T>(path: string, body: unknown, locale?: string) =>
    request<T>(path, { method: 'POST', body, locale }),

  patch: <T>(path: string, body: unknown, locale?: string) =>
    request<T>(path, { method: 'PATCH', body, locale }),

  del: <T>(path: string, locale?: string) =>
    request<T>(path, { method: 'DELETE', locale }),
};
