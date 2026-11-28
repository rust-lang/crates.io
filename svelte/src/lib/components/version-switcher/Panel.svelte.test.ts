import type { ResolvedPathname } from '$app/types';
import type { ComponentProps } from 'svelte';

import { describe, expect, it, onTestFinished, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import Panel from './Panel.svelte';

function createVersions(count: number) {
  return Array.from({ length: count }, (_, i) => ({
    num: `1.0.${count - 1 - i}`,
    created_at: '2026-05-04T00:00:00Z',
    yanked: false,
  }));
}

async function renderPanel(props: Partial<ComponentProps<typeof Panel>> = {}) {
  // A fixed height makes the list scroll, so that the end of a long list is out of view.
  let target = document.createElement('div');
  target.style.cssText = 'display: flex; flex-direction: column; height: 200px';
  document.body.append(target);
  onTestFinished(() => target.remove());

  let onLoadMore = vi.fn();
  await render(Panel, {
    target,
    props: {
      versions: [],
      isComplete: false,
      error: false,
      currentNum: '1.0.1',
      versionHref: num => `/crates/foo/${num}` as ResolvedPathname,
      onLoadMore,
      ...props,
    },
  });
  return { onLoadMore };
}

function nextFrame() {
  return new Promise(resolve => requestAnimationFrame(resolve));
}

describe('version-switcher/Panel', () => {
  let rows = page.getByCSS('[data-test-row]');
  let loading = page.getByCSS('[data-test-loading]');
  let errorMessage = page.getByCSS('[data-test-error]');

  it('shows a row per version', async () => {
    await renderPanel({ versions: createVersions(3), isComplete: true, defaultNum: '1.0.2' });

    await expect.element(rows.nth(0)).toHaveAttribute('href', '/crates/foo/1.0.2');
    await expect.element(rows.nth(0)).toHaveTextContent('1.0.2 Default May 4, 2026');
    await expect.element(rows.nth(1)).toHaveAttribute('aria-current', 'page');
    await expect.element(rows.nth(2)).toHaveTextContent('1.0.0 May 4, 2026');
    expect(rows.all()).toHaveLength(3);
    await expect.element(loading).not.toBeInTheDocument();
  });

  it('calls `onSelect` when a version is clicked', async () => {
    let onSelect = vi.fn();
    // Fragment links keep the test page from navigating away.
    let versionHref = (num: string) => `#${num}` as ResolvedPathname;
    await renderPanel({ versions: createVersions(3), isComplete: true, versionHref, onSelect });

    await userEvent.click(rows.nth(1));
    expect(onSelect).toHaveBeenCalledOnce();
  });

  it('loads the first page right away', async () => {
    let { onLoadMore } = await renderPanel();

    await expect.element(loading).toHaveTextContent('Loading…');
    await expect.poll(() => onLoadMore).toHaveBeenCalledOnce();
  });

  it('loads the next page when the end of the list scrolls into view', async () => {
    let { onLoadMore } = await renderPanel({ versions: createVersions(50) });

    // Intersection observer callbacks run before the next frame is rendered.
    await nextFrame();
    await nextFrame();
    expect(onLoadMore).not.toHaveBeenCalled();

    loading.element().scrollIntoView();
    await expect.poll(() => onLoadMore).toHaveBeenCalledOnce();
  });

  it('stops loading once all versions are loaded', async () => {
    let { onLoadMore } = await renderPanel({ versions: createVersions(3), isComplete: true });

    // Intersection observer callbacks run before the next frame is rendered.
    await nextFrame();
    await nextFrame();
    expect(onLoadMore).not.toHaveBeenCalled();
  });

  it('offers to try again after an error', async () => {
    let { onLoadMore } = await renderPanel({ versions: createVersions(3), error: true });

    await expect.element(errorMessage).toHaveTextContent('Failed to load versions. Try again');
    await expect.element(loading).not.toBeInTheDocument();
    expect(onLoadMore).not.toHaveBeenCalled();

    await page.getByRole('button', { name: 'Try again' }).click();
    expect(onLoadMore).toHaveBeenCalledOnce();
  });
});
