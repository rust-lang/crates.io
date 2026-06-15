import type { NativeReplacement } from '$lib/data/native-replacements';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import NativeReplacementBanner from './NativeReplacementBanner.svelte';

const STD_URL = 'https://doc.rust-lang.org/std/sync/struct.LazyLock.html';

const replacement: NativeReplacement = {
  description: 'Use `std::sync::LazyLock` (Rust 1.80) instead.',
  url: STD_URL,
};

describe('NativeReplacementBanner', () => {
  it('renders the description Markdown and a "Learn more" link', async () => {
    render(NativeReplacementBanner, { replacement });

    let banner = page.getByCSS('[data-test-native-replacement-banner]');
    await expect.element(banner).toBeVisible();
    await expect.element(banner).toHaveTextContent('You might not need this dependency.');

    let code = page.getByCSS('[data-test-native-replacement-banner] code');
    expect(code.elements()).toHaveLength(1);
    await expect.element(code).toHaveTextContent('std::sync::LazyLock');

    let link = page.getByRole('link', { name: 'Learn more' });
    await expect.element(link).toHaveAttribute('href', STD_URL);
    await expect.element(link).toHaveAttribute('target', '_blank');
    await expect.element(link).toHaveAttribute('rel', 'noopener noreferrer');
  });
});
