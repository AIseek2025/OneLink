import { bffUrl, BFF_BASE_URL, API_PREFIX } from '../services/config';

describe('config', () => {
  it('constructs BFF URL correctly', () => {
    const url = bffUrl('/auth/login');
    expect(url).toBe(`${BFF_BASE_URL}${API_PREFIX}/auth/login`);
  });

  it('uses /api/v1/bff prefix (frozen contract boundary)', () => {
    expect(API_PREFIX).toBe('/api/v1/bff');
  });

  it('does not mix /api/v1/app paths', () => {
    expect(API_PREFIX).not.toContain('/app');
  });
});