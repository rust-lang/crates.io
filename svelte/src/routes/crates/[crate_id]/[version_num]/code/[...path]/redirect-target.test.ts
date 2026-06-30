import type { ManifestFile } from '$lib/utils/zip-archive';

import { describe, expect, it } from 'vitest';

import { redirectTarget } from './redirect-target';

function files(...paths: string[]): ManifestFile[] {
  return paths.map(path => ({
    path,
    data_offset: 0,
    compressed_size: 0,
    uncompressed_size: 0,
    compression: 'deflate',
    sha256: '',
  }));
}

describe('redirectTarget()', () => {
  it('returns undefined when the path is already a file', () => {
    expect(redirectTarget(files('src/lib.rs', 'Cargo.toml'), 'Cargo.toml')).toBeUndefined();
  });

  describe('default file (no path)', () => {
    it('prefers src/lib.rs', () => {
      let target = redirectTarget(files('Cargo.toml', 'src/main.rs', 'src/lib.rs'), '');
      expect(target?.path).toBe('src/lib.rs');
    });

    it('falls back to src/main.rs when src/lib.rs is absent', () => {
      let target = redirectTarget(files('Cargo.toml', 'src/main.rs'), '');
      expect(target?.path).toBe('src/main.rs');
    });

    it('falls back to Cargo.toml when neither src file exists', () => {
      let target = redirectTarget(files('Cargo.toml', 'README.md'), '');
      expect(target?.path).toBe('Cargo.toml');
    });

    it('falls back to the lexicographically first file when no priority file matches', () => {
      let target = redirectTarget(files('src/util.rs', 'README.md'), '');
      expect(target?.path).toBe('README.md');
    });

    it('returns undefined when the archive has no files', () => {
      expect(redirectTarget(files(), '')).toBeUndefined();
    });
  });

  describe('directory path', () => {
    it('redirects to the lexicographically first file in the directory', () => {
      let target = redirectTarget(files('src/util.rs', 'src/de.rs', 'src/lib.rs'), 'src');
      expect(target?.path).toBe('src/de.rs');
    });

    it('treats a trailing-slash path as a directory', () => {
      let target = redirectTarget(files('docs/guide.md', 'docs/api.md'), 'docs/');
      expect(target?.path).toBe('docs/api.md');
    });

    it('does not match files whose name merely shares the path as a prefix', () => {
      let target = redirectTarget(files('src_helpers.rs', 'src/lib.rs'), 'src');
      expect(target?.path).toBe('src/lib.rs');
    });

    it('returns undefined for a path that is neither a file nor a directory', () => {
      expect(redirectTarget(files('src/lib.rs'), 'does/not/exist')).toBeUndefined();
    });
  });
});
