import { bffClient, setAuthToken, getAuthToken, BffClientError } from '../services/bffClient';

describe('bffClient', () => {
  it('manages auth token', () => {
    setAuthToken(null);
    expect(getAuthToken()).toBeNull();
    setAuthToken('test-token');
    expect(getAuthToken()).toBe('test-token');
    setAuthToken(null);
    expect(getAuthToken()).toBeNull();
  });

  it('exposes get, post, patch, del methods', () => {
    expect(typeof bffClient.get).toBe('function');
    expect(typeof bffClient.post).toBe('function');
    expect(typeof bffClient.patch).toBe('function');
    expect(typeof bffClient.del).toBe('function');
  });

  it('BffClientError carries status, code, traceId', () => {
    const err = new BffClientError({
      status: 401,
      code: 'AUTH_EXPIRED',
      message: 'Session expired',
      trace_id: 'trace-123',
    });
    expect(err.status).toBe(401);
    expect(err.code).toBe('AUTH_EXPIRED');
    expect(err.traceId).toBe('trace-123');
    expect(err.message).toBe('Session expired');
    expect(err.name).toBe('BffClientError');
  });
});