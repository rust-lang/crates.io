import { describe, expect, it } from 'vitest';

import { isRetryableStatus } from './api';

describe('isRetryableStatus', () => {
  it.each([429, 500, 502, 503, 504])('treats %i as retryable', status => {
    expect(isRetryableStatus(status)).toBe(true);
  });

  it.each([400, 401, 403, 404, 422])('treats %i as not retryable', status => {
    expect(isRetryableStatus(status)).toBe(false);
  });
});
