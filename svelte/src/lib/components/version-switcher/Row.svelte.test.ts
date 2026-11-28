import type { ResolvedPathname } from '$app/types';

import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import Row from './Row.svelte';

// May 3 in UTC, but already May 4 in the `Pacific/Auckland` time zone of the browser tests
const VERSION = { num: '1.2.3', created_at: '2026-05-03T18:00:00Z', yanked: false };

describe('version-switcher/Row', () => {
  let row = page.getByCSS('[data-test-row]');
  let defaultLabel = page.getByCSS('[data-test-default]');
  let date = page.getByCSS('[data-test-date]');

  it('links to the version and shows its publish date', async () => {
    await render(Row, { version: VERSION, href: '/crates/foo/1.2.3' });

    await expect.element(row).toHaveAttribute('href', '/crates/foo/1.2.3');
    await expect.element(row).toHaveTextContent('1.2.3 May 4, 2026');
    await expect.element(row).toHaveAccessibleName('1.2.3, published May 4, 2026');
    await expect.element(row).not.toHaveAttribute('aria-current');
    await expect.element(row).not.toHaveClass('current');
    await expect.element(row).not.toHaveClass('yanked');
    await expect.element(date).toHaveTextContent('May 4, 2026');
    await expect.element(date).toHaveAttribute('datetime', '2026-05-03T18:00:00Z');
    await expect.element(defaultLabel).not.toBeInTheDocument();
  });

  it('marks the current version', async () => {
    await render(Row, { version: VERSION, href: '/crates/foo/1.2.3', isCurrent: true });

    await expect.element(row).toHaveAttribute('aria-current', 'page');
    await expect.element(row).toHaveClass('current');
  });

  it('labels the default version', async () => {
    await render(Row, { version: VERSION, href: '/crates/foo/1.2.3', isDefault: true });

    await expect.element(defaultLabel).toHaveTextContent('Default');
    await expect.element(row).toHaveTextContent('1.2.3 Default May 4, 2026');
    await expect.element(row).toHaveAccessibleName('1.2.3, default version, published May 4, 2026');
  });

  it('shows "yanked" instead of the publish date for yanked versions', async () => {
    await render(Row, { version: { ...VERSION, yanked: true }, href: '/crates/foo/1.2.3' });

    await expect.element(row).toHaveClass('yanked');
    await expect.element(row).toHaveTextContent('1.2.3 yanked');
    await expect.element(row).toHaveAccessibleName('1.2.3, yanked');
    await expect.element(date).not.toBeInTheDocument();
  });

  it('calls `onClick` when clicked', async () => {
    let onClick = vi.fn();
    // A fragment link keeps the test page from navigating away.
    await render(Row, { version: VERSION, href: '#1.2.3' as ResolvedPathname, onClick });

    await userEvent.click(row);
    expect(onClick).toHaveBeenCalledOnce();
  });
});
