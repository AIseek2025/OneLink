import { useState, useCallback } from 'react';
import { bffClient, BffClientError } from '../services/bffClient';

type RequestStatus = 'idle' | 'loading' | 'success' | 'error';

interface UseBffRequestResult<T> {
  data: T | null;
  status: RequestStatus;
  error: string;
  request: (path: string, method?: 'GET' | 'POST' | 'PATCH' | 'DELETE', body?: unknown) => Promise<T>;
  reset: () => void;
}

export function useBffRequest<T>(): UseBffRequestResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [status, setStatus] = useState<RequestStatus>('idle');
  const [error, setError] = useState('');

  const request = useCallback(async (
    path: string,
    method: 'GET' | 'POST' | 'PATCH' | 'DELETE' = 'GET',
    body?: unknown,
  ): Promise<T> => {
    setStatus('loading');
    setError('');
    try {
      let result: T;
      switch (method) {
        case 'POST':
          result = await bffClient.post<T>(path, body);
          break;
        case 'PATCH':
          result = await bffClient.patch<T>(path, body);
          break;
        case 'DELETE':
          result = await bffClient.del<T>(path);
          break;
        default:
          result = await bffClient.get<T>(path);
      }
      setData(result);
      setStatus('success');
      return result;
    } catch (e) {
      const msg = e instanceof BffClientError ? e.message : '请求失败';
      setError(msg);
      setStatus('error');
      throw e;
    }
  }, []);

  const reset = useCallback(() => {
    setData(null);
    setStatus('idle');
    setError('');
  }, []);

  return { data, status, error, request, reset };
}
