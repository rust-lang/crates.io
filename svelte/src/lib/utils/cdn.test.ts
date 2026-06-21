import { describe, expect, it } from 'vitest';

import { cdnBase } from './cdn';

describe('cdnBase', () => {
  it('returns the reported CDN host', () => {
    expect(cdnBase({ cdn_base: 'https://static.example.com' })).toBe('https://static.example.com');
  });

  it('returns an empty string for same-origin (no CDN)', () => {
    expect(cdnBase({ cdn_base: '' })).toBe('');
  });

  it('falls back to the production CDN when the field is absent', () => {
    expect(cdnBase({})).toBe('https://static.crates.io');
  });
});
