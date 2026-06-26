import type { Unmaintained } from '$lib/utils/rustsec';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import UnmaintainedBanner from './UnmaintainedBanner.svelte';

const ADVISORY_URL = 'https://rustsec.org/advisories/RUSTSEC-2021-0139.html';

const unmaintained: Unmaintained = {
  id: 'RUSTSEC-2021-0139',
  url: ADVISORY_URL,
};

describe('UnmaintainedBanner', () => {
  it('renders the explanation and a link to the advisory', async () => {
    render(UnmaintainedBanner, { unmaintained });

    let banner = page.getByCSS('[data-test-unmaintained-banner]');
    await expect.element(banner).toBeVisible();
    await expect.element(banner).toHaveTextContent('This crate has been marked as unmaintained');

    let link = page.getByRole('link', { name: 'RUSTSEC-2021-0139' });
    await expect.element(link).toHaveAttribute('href', ADVISORY_URL);
    await expect.element(link).toHaveAttribute('target', '_blank');
    await expect.element(link).toHaveAttribute('rel', 'noopener noreferrer');
  });
});
