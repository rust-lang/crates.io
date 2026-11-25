import type { ResolvedPathname } from '$app/types';

import { db, handlers } from '@crates-io/msw';
import { afterEach, describe, expect } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import { test } from '../../../test/msw';
import { VersionStore } from './version-store.svelte';
import VersionSwitcher from './VersionSwitcher.svelte';

afterEach(() => db.reset());

async function renderSwitcher() {
  let crate = await db.crate.create({ name: 'foo' });
  for (let num of ['1.0.0', '1.1.0', '1.2.0']) {
    await db.version.create({ crate, num });
  }

  // Fragment links keep the test page from navigating away.
  let versionHref = (num: string) => `#${num}` as ResolvedPathname;
  let store = new VersionStore('foo');
  await render(VersionSwitcher, { store, crateName: 'foo', currentNum: '1.1.0', versionHref });
}

describe('version-switcher/VersionSwitcher', () => {
  let trigger = page.getByRole('button', { name: 'v1.1.0' });
  let rows = page.getByCSS('[data-test-row]');
  let dialog = page.getByRole('dialog', { name: 'foo versions' });

  test('shows the versions in a popover on wide screens', async ({ worker }) => {
    worker.use(...handlers);
    await page.viewport(1280, 800);
    await renderSwitcher();

    await expect.element(trigger).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(trigger);
    await expect.element(trigger).toHaveAttribute('aria-expanded', 'true');
    await expect.poll(() => rows.all()).toHaveLength(3);
    await expect.element(rows.nth(1)).toHaveAttribute('aria-current', 'page');
    await expect.element(dialog).not.toBeInTheDocument();

    await userEvent.click(rows.nth(0));
    await expect.element(rows.nth(0)).not.toBeInTheDocument();
    await expect.element(trigger).toHaveAttribute('aria-expanded', 'false');
  });

  test('shows the versions in a fullscreen dialog on narrow screens', async ({ worker }) => {
    worker.use(...handlers);
    await page.viewport(400, 800);
    await renderSwitcher();

    await userEvent.click(trigger);
    await expect.element(dialog).toBeVisible();
    await expect.poll(() => rows.all()).toHaveLength(3);

    await userEvent.click(rows.nth(0));
    await expect.element(dialog).not.toBeInTheDocument();
  });
});
