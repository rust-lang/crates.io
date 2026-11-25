import { describe, expect, it } from 'vitest';

import { versionHref } from './version-href';

describe('versionHref()', () => {
  it('links to the version page from the crate page', () => {
    expect(versionHref('/crates/[crate_id]', {}, 'foo', '1.2.3')).toBe('/crates/foo/1.2.3');
  });

  it('links to the version page from another version page', () => {
    expect(versionHref('/crates/[crate_id]/[version_num]', {}, 'foo', '1.2.3')).toBe('/crates/foo/1.2.3');
  });

  it('keeps the dependencies page', () => {
    let href = versionHref('/crates/[crate_id]/[version_num]/dependencies', {}, 'foo', '1.2.3');
    expect(href).toBe('/crates/foo/1.2.3/dependencies');
  });

  it('keeps the file path in the code browser', () => {
    let params = { path: 'src/lib.rs' };
    let href = versionHref('/crates/[crate_id]/[version_num]/code/[...path]', params, 'foo', '1.2.3');
    expect(href).toBe('/crates/foo/1.2.3/code/src/lib.rs');
  });

  it('links to the root of the code browser', () => {
    let href = versionHref('/crates/[crate_id]/[version_num]/code/[...path]', { path: '' }, 'foo', '1.2.3');
    expect(href).toBe('/crates/foo/1.2.3/code');
  });
});
