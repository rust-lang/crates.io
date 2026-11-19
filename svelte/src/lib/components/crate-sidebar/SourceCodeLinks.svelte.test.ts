import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import SourceCodeLinks from './SourceCodeLinks.svelte';

describe('SourceCodeLinks', () => {
  it('links to the code browser and the files on the CDN', async () => {
    await render(SourceCodeLinks, { crate: 'serde', version: '1.0.228', cdnBase: 'https://static.crates.io' });

    await expect
      .element(page.getByCSS('[data-test-browse-link]'))
      .toHaveAttribute('href', '/crates/serde/1.0.228/code');
    await expect
      .element(page.getByCSS('[data-test-crate-link]'))
      .toHaveAttribute('href', 'https://static.crates.io/crates/serde/serde-1.0.228.crate');
    await expect
      .element(page.getByCSS('[data-test-zip-link]'))
      .toHaveAttribute('href', 'https://static.crates.io/crates/serde/serde-1.0.228.zip');
  });

  it('encodes build metadata in the download URLs', async () => {
    await render(SourceCodeLinks, {
      crate: 'libgit2-sys',
      version: '0.16.2+1.7.2',
      cdnBase: 'https://static.crates.io',
    });

    await expect
      .element(page.getByCSS('[data-test-crate-link]'))
      .toHaveAttribute('href', 'https://static.crates.io/crates/libgit2-sys/libgit2-sys-0.16.2%2B1.7.2.crate');
    await expect
      .element(page.getByCSS('[data-test-zip-link]'))
      .toHaveAttribute('href', 'https://static.crates.io/crates/libgit2-sys/libgit2-sys-0.16.2%2B1.7.2.zip');
  });
});
