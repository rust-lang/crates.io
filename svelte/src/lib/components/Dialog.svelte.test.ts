import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import DialogTestWrapper from './DialogTestWrapper.svelte';

describe('Dialog', () => {
  it('opens as a modal dialog with a title', async () => {
    await render(DialogTestWrapper);

    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();

    await userEvent.click(page.getByCSS('[data-test-trigger]'));

    let dialog = page.getByRole('dialog', { name: 'Example dialog' });
    await expect.element(dialog).toBeVisible();
    await expect.element(dialog.getByCSS('[data-test-content]')).toHaveTextContent('Dialog content');
    expect(dialog.element().matches(':modal')).toBe(true);
  });

  it('opens and closes via its invoker button', async () => {
    await render(DialogTestWrapper, { useInvoker: true });

    await userEvent.click(page.getByCSS('[data-test-trigger]'));
    let dialog = page.getByRole('dialog', { name: 'Example dialog' });
    await expect.element(dialog.getByCSS('[data-test-content]')).toHaveTextContent('Dialog content');

    await userEvent.click(page.getByRole('button', { name: 'Close' }));
    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
    await expect.element(page.getByCSS('[data-test-trigger]')).toHaveFocus();

    await userEvent.click(page.getByCSS('[data-test-trigger]'));
    await expect.element(dialog).toBeVisible();
  });

  it('closes via the close button and returns focus to the trigger', async () => {
    await render(DialogTestWrapper);

    await userEvent.click(page.getByCSS('[data-test-trigger]'));
    await expect.element(page.getByRole('dialog')).toBeVisible();

    await userEvent.click(page.getByRole('button', { name: 'Close' }));

    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
    await expect.element(page.getByCSS('[data-test-content]')).not.toBeInTheDocument();
    await expect.element(page.getByCSS('[data-test-trigger]')).toHaveFocus();
  });

  it('closes on Escape and returns focus to the trigger', async () => {
    await render(DialogTestWrapper);

    await userEvent.click(page.getByCSS('[data-test-trigger]'));
    await expect.element(page.getByRole('dialog')).toBeVisible();

    await userEvent.keyboard('{Escape}');

    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
    await expect.element(page.getByCSS('[data-test-content]')).not.toBeInTheDocument();
    await expect.element(page.getByCSS('[data-test-trigger]')).toHaveFocus();
  });

  it('can be opened again after closing', async () => {
    await render(DialogTestWrapper);

    await userEvent.click(page.getByCSS('[data-test-trigger]'));
    await userEvent.keyboard('{Escape}');
    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();

    await userEvent.click(page.getByCSS('[data-test-trigger]'));

    await expect.element(page.getByRole('dialog')).toBeVisible();
  });

  it('moves the dialog out of its parent and into the body', async () => {
    let { unmount } = await render(DialogTestWrapper);

    let dialog = document.querySelector('dialog')!;
    expect(dialog.parentElement).toBe(document.body);
    expect(document.querySelector('[data-test-heading]')!.childElementCount).toBe(0);

    unmount();

    expect(dialog.isConnected).toBe(false);
  });

  it('keeps the page from scrolling while open', async () => {
    await render(DialogTestWrapper);

    await userEvent.click(page.getByCSS('[data-test-trigger]'));
    await expect.element(page.getByRole('dialog')).toBeVisible();

    let root = document.documentElement;
    expect(getComputedStyle(root).overflow).toBe('hidden');

    await userEvent.click(page.getByRole('button', { name: 'Close' }));
    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();

    expect(getComputedStyle(root).overflow).toBe('visible');
  });

  it('centers the dialog by default', async () => {
    await render(DialogTestWrapper);

    await userEvent.click(page.getByCSS('[data-test-trigger]'));

    let dialog = page.getByRole('dialog');
    await expect.element(dialog).toBeVisible();

    let rect = dialog.element().getBoundingClientRect();
    expect(rect.left + rect.width / 2).toBeCloseTo(window.innerWidth / 2, 0);
    expect(rect.top + rect.height / 2).toBeCloseTo(window.innerHeight / 2, 0);
    expect(rect.width).toBeLessThan(window.innerWidth);
  });

  it('covers the whole viewport when fullscreen', async () => {
    await render(DialogTestWrapper, { fullscreen: true });

    await userEvent.click(page.getByCSS('[data-test-trigger]'));

    let dialog = page.getByRole('dialog');
    await expect.element(dialog).toBeVisible();

    let rect = dialog.element().getBoundingClientRect();
    expect(rect).toMatchObject({ top: 0, left: 0, width: window.innerWidth, height: window.innerHeight });
  });
});
