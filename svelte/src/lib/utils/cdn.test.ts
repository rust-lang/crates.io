import { describe, expect, it } from 'vitest';

import { cdnBase, crateFileUrl } from './cdn';

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

describe('crateFileUrl', () => {
  it('returns the URL of the `.crate` file', () => {
    expect(crateFileUrl('https://static.crates.io', 'serde', '1.0.228', 'crate')).toBe(
      'https://static.crates.io/crates/serde/serde-1.0.228.crate',
    );
  });

  it('returns the URL of the `.zip` file', () => {
    expect(crateFileUrl('https://static.crates.io', 'serde', '1.0.228', 'zip')).toBe(
      'https://static.crates.io/crates/serde/serde-1.0.228.zip',
    );
  });

  it('encodes build metadata in the version', () => {
    expect(crateFileUrl('', 'libgit2-sys', '0.16.2+1.7.2', 'crate')).toBe(
      '/crates/libgit2-sys/libgit2-sys-0.16.2%2B1.7.2.crate',
    );
  });
});
