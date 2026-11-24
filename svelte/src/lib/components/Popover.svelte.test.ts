import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import PopoverTestWrapper from './PopoverTestWrapper.svelte';

describe('Popover', () => {
  let trigger = page.getByCSS('[data-test-trigger]');
  let popover = page.getByCSS('[data-test-popover]');
  let item = page.getByCSS('[data-test-item]');
  let outside = page.getByCSS('[data-test-outside]');

  function rects() {
    return {
      anchor: trigger.element().getBoundingClientRect(),
      popover: popover.element().getBoundingClientRect(),
    };
  }

  it('toggles when the anchor is clicked', async () => {
    await render(PopoverTestWrapper);
    await expect.element(item).not.toBeInTheDocument();

    await userEvent.click(trigger);
    await expect.element(item).toBeVisible();
    expect(popover.element().matches(':popover-open')).toBe(true);

    await userEvent.click(trigger);
    await expect.element(item).not.toBeInTheDocument();
    await expect.element(popover).not.toBeVisible();
  });

  it('opens and closes when `open` changes', async () => {
    await render(PopoverTestWrapper);
    let openButton = page.getByCSS('[data-test-open]');

    await userEvent.click(openButton);
    await expect.element(item).toBeVisible();

    await userEvent.click(page.getByCSS('[data-test-close]'));
    await expect.element(item).not.toBeInTheDocument();
    await expect.element(openButton).toHaveFocus();
  });

  it('updates `open` when the browser closes the popover', async () => {
    await render(PopoverTestWrapper);
    await userEvent.click(trigger);
    await userEvent.keyboard('{Escape}');
    await expect.element(item).not.toBeInTheDocument();

    // reopening through `open` only works if it was reset to `false`
    await userEvent.click(page.getByCSS('[data-test-open]'));
    await expect.element(item).toBeVisible();
  });

  it('places the popover below the start of the anchor by default', async () => {
    await render(PopoverTestWrapper);
    await userEvent.click(trigger);

    // popover is directly below the anchor, aligned to its left edge
    await expect.poll(() => rects().popover.top - rects().anchor.bottom).toBeCloseTo(0);
    expect(rects().popover.left - rects().anchor.left).toBeCloseTo(0);
  });

  it('places the popover according to `placement`', async () => {
    await render(PopoverTestWrapper, { placement: 'bottom-end' });
    await userEvent.click(trigger);

    // popover is aligned to the right edge of the anchor
    await expect.poll(() => rects().popover.right - rects().anchor.right).toBeCloseTo(0, 1);
  });

  it('closes when clicking outside', async () => {
    await render(PopoverTestWrapper);
    await userEvent.click(trigger);
    await expect.element(item).toBeVisible();

    await userEvent.click(outside);
    await expect.element(item).not.toBeInTheDocument();
  });

  it('stays open when clicking inside', async () => {
    await render(PopoverTestWrapper);
    await userEvent.click(trigger);

    await userEvent.click(item);
    await expect.element(item).toBeVisible();
  });

  it('closes when the anchor is hidden by a media query', async () => {
    let style = document.createElement('style');
    style.textContent = '@media (max-width: 500px) { [data-test-heading] { display: none } }';
    document.head.append(style);
    let { innerWidth, innerHeight } = globalThis;
    try {
      await page.viewport(800, 600);
      await render(PopoverTestWrapper);
      await userEvent.click(trigger);
      await expect.element(item).toBeVisible();

      await page.viewport(400, 600);
      await expect.element(item).not.toBeInTheDocument();
      await expect.element(popover).not.toBeVisible();
    } finally {
      await page.viewport(innerWidth, innerHeight);
      style.remove();
    }
  });

  it('renders into the body instead of its parent', async () => {
    let { unmount } = await render(PopoverTestWrapper);
    await userEvent.click(trigger);
    await expect.element(item).toBeVisible();

    let heading = document.querySelector('[data-test-heading]')!;
    expect(popover.element().parentElement).toBe(document.body);
    expect(getComputedStyle(item.element()).fontSize).not.toBe(getComputedStyle(heading).fontSize);

    unmount();
    await expect.element(popover).not.toBeInTheDocument();
  });

  it('follows the anchor in the Tab order', async () => {
    await render(PopoverTestWrapper);
    await userEvent.click(trigger);
    await expect.element(item).toBeVisible();

    await userEvent.tab();
    await expect.element(item).toHaveFocus();
    await userEvent.tab();
    await userEvent.tab();
    await expect.element(outside).toHaveFocus();
  });

  it('closes on Escape and returns focus to the anchor', async () => {
    await render(PopoverTestWrapper);
    await userEvent.click(trigger);
    item.element().focus();

    await userEvent.keyboard('{Escape}');
    await expect.element(item).not.toBeInTheDocument();
    await expect.element(trigger).toHaveFocus();
  });

  it('returns focus on Escape to the element that was focused before opening', async () => {
    await render(PopoverTestWrapper);
    outside.element().focus();
    // A scripted click doesn't focus the anchor.
    (trigger.element() as HTMLButtonElement).click();
    await expect.element(item).toBeVisible();
    item.element().focus();

    await userEvent.keyboard('{Escape}');
    await expect.element(item).not.toBeInTheDocument();
    await expect.element(outside).toHaveFocus();
  });

  it('closes on Escape without moving focus that is outside the popover', async () => {
    await render(PopoverTestWrapper);
    await userEvent.click(trigger);
    outside.element().focus();

    await userEvent.keyboard('{Escape}');
    await expect.element(item).not.toBeInTheDocument();
    await expect.element(outside).toHaveFocus();
  });
});
