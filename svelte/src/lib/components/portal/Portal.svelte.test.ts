import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import PortalTestWrapper from './PortalTestWrapper.svelte';

describe('Portal', () => {
  it('renders its children at the end of the body', async () => {
    await render(PortalTestWrapper, { label: 'Hello' });

    await expect.element(page.getByCSS('[data-test-label]')).toHaveTextContent('Hello');
    expect(document.querySelector('[data-test-label]')!.parentElement).toBe(document.body);
    expect(document.querySelector('[data-test-parent]')!.childElementCount).toBe(0);
  });

  it('updates its children when the parent state changes', async () => {
    let { rerender } = await render(PortalTestWrapper, { label: 'Hello' });

    await rerender({ label: 'World' });

    await expect.element(page.getByCSS('[data-test-label]')).toHaveTextContent('World');
  });

  it('passes the parent contexts to its children', async () => {
    await render(PortalTestWrapper, { label: 'Hello' });

    await expect.element(page.getByCSS('[data-test-context]')).toHaveTextContent('context value');
  });

  it('removes its children when unmounted', async () => {
    let { unmount } = await render(PortalTestWrapper, { label: 'Hello' });
    await expect.element(page.getByCSS('[data-test-label]')).toBeInTheDocument();

    unmount();

    expect(document.querySelector('[data-test-label]')).toBeNull();
  });
});
