import { describe, expect, it } from 'vitest';

import { languageForPath } from './syntax-language';

describe('languageForPath', () => {
  it('maps a file extension to its Shiki language id', () => {
    expect(languageForPath('src/lib.rs')).toBe('rust');
    expect(languageForPath('Cargo.toml')).toBe('toml');
    expect(languageForPath('README.md')).toBe('markdown');
    expect(languageForPath('build.js')).toBe('javascript');
  });

  it('resolves the language from the file name, ignoring directories', () => {
    expect(languageForPath('src/core/de.rs')).toBe('rust');
  });

  it('matches extensions case-insensitively', () => {
    expect(languageForPath('SRC/LIB.RS')).toBe('rust');
  });

  it('maps well-known file names whose extension is not a language', () => {
    expect(languageForPath('Cargo.lock')).toBe('toml');
    expect(languageForPath('vendor/Cargo.toml.orig')).toBe('toml');
  });

  it('falls back to plain text for an unknown extension', () => {
    expect(languageForPath('assets/icon.bin')).toBe('text');
  });

  it('falls back to plain text for a file without an extension', () => {
    expect(languageForPath('LICENSE')).toBe('text');
    expect(languageForPath('Makefile')).toBe('text');
  });
});
