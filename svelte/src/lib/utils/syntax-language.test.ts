import { getFiletypeFromFileName } from '@pierre/diffs';
import { describe, expect, it } from 'vitest';

import { registerCustomExtensions } from './syntax-language';

describe('registerCustomExtensions', () => {
  registerCustomExtensions();

  it('highlights `Cargo.lock` as TOML', () => {
    expect(getFiletypeFromFileName('Cargo.lock')).toBe('toml');
  });

  it('highlights `*.toml.orig` as TOML, ignoring directories', () => {
    expect(getFiletypeFromFileName('Cargo.toml.orig')).toBe('toml');
    expect(getFiletypeFromFileName('vendor/Cargo.toml.orig')).toBe('toml');
  });

  it('highlights SVG files as XML', () => {
    expect(getFiletypeFromFileName('icon.svg')).toBe('xml');
  });

  it('leaves other files to the highlighter to infer', () => {
    expect(getFiletypeFromFileName('src/lib.rs')).toBe('rust');
  });
});
