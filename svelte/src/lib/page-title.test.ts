import { describe, expect, it } from 'vitest';

import { PageTitleState } from './page-title.svelte';

describe('PageTitleState', () => {
  it('returns base title when no segments are pushed', () => {
    let state = new PageTitleState();
    expect(state.title).toBe('crates.io: Rust Package Registry');
  });

  it('composes a single segment with the base title', () => {
    let state = new PageTitleState();
    state.push('Crates');
    expect(state.title).toBe('Crates - crates.io: Rust Package Registry');
  });

  it('composes multiple segments in reverse order (deepest first)', () => {
    let state = new PageTitleState();
    state.push('serde');
    state.push('Versions');
    expect(state.title).toBe('Versions - serde - crates.io: Rust Package Registry');
  });

  it('removes segment when cleanup function is called', () => {
    let state = new PageTitleState();
    let cleanup = state.push('Crates');
    expect(state.title).toBe('Crates - crates.io: Rust Package Registry');

    cleanup();
    expect(state.title).toBe('crates.io: Rust Package Registry');
  });

  it('removes only the specific token when cleanup is called with duplicate titles', () => {
    let state = new PageTitleState();
    let cleanup1 = state.push('Same');
    state.push('Same');
    expect(state.title).toBe('Same - Same - crates.io: Rust Package Registry');

    cleanup1();
    expect(state.title).toBe('Same - crates.io: Rust Package Registry');
  });

  it('handles push and cleanup in nested layout/page order', () => {
    let state = new PageTitleState();

    // Layout mounts and pushes "serde"
    let cleanupLayout = state.push('serde');
    expect(state.title).toBe('serde - crates.io: Rust Package Registry');

    // Page mounts and pushes "Versions"
    let cleanupPage = state.push('Versions');
    expect(state.title).toBe('Versions - serde - crates.io: Rust Package Registry');

    // Page unmounts
    cleanupPage();
    expect(state.title).toBe('serde - crates.io: Rust Package Registry');

    // Layout unmounts
    cleanupLayout();
    expect(state.title).toBe('crates.io: Rust Package Registry');
  });

  it('handles page navigation (old page cleanup, new page push)', () => {
    let state = new PageTitleState();

    // First page
    let cleanup1 = state.push('Crates');
    expect(state.title).toBe('Crates - crates.io: Rust Package Registry');

    // Navigate: old page unmounts, new page mounts
    cleanup1();
    state.push('Categories');
    expect(state.title).toBe('Categories - crates.io: Rust Package Registry');
  });

  it('handles three levels of nesting', () => {
    let state = new PageTitleState();
    state.push('Settings');
    state.push('Tokens');
    state.push('New');
    expect(state.title).toBe('New - Tokens - Settings - crates.io: Rust Package Registry');
  });
});
