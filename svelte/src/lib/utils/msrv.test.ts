import { describe, expect, it } from 'vitest';

import { normalizeMsrv } from './msrv';

describe('normalizeMsrv', () => {
  it('appends `.0` when the rust-version has two components', () => {
    expect(normalizeMsrv('1.69')).toBe('1.69.0');
    expect(normalizeMsrv('1.23')).toBe('1.23.0');
  });

  it('returns three-component versions unchanged', () => {
    expect(normalizeMsrv('1.69.0')).toBe('1.69.0');
    expect(normalizeMsrv('1.69.2')).toBe('1.69.2');
  });
});
