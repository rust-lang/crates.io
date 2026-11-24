import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import TooltipTestWrapper from './TooltipTestWrapper.svelte';

describe('Tooltip', () => {
  it('shows the tooltip on hover', async () => {
    await render(TooltipTestWrapper, { text: 'short', width: '500px' });

    await userEvent.hover(page.getByCSS('[data-test-anchor]'));

    await expect.element(page.getByCSS('.tooltip')).toBeVisible();
  });

  describe('positioning', () => {
    function rects() {
      let anchor = document.querySelector('[data-test-anchor]')!.getBoundingClientRect();
      let tooltip = document.querySelector('.tooltip')!.getBoundingClientRect();
      return { anchor, tooltip };
    }

    function horizontalCenterOffset() {
      let { anchor, tooltip } = rects();
      return tooltip.left + tooltip.width / 2 - (anchor.left + anchor.width / 2);
    }

    it('centers the tooltip above the anchor by default', async () => {
      await render(TooltipTestWrapper, { text: 'short', width: '200px' });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));
      await expect.element(page.getByCSS('.tooltip')).toBeVisible();

      // tooltip is above the anchor
      await expect.poll(() => rects().anchor.top - rects().tooltip.bottom).toBeCloseTo(5);
      expect(horizontalCenterOffset()).toBeCloseTo(0);
    });

    it('places the tooltip on the configured side', async () => {
      await render(TooltipTestWrapper, { text: 'short', width: '200px', side: 'bottom' });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));
      await expect.element(page.getByCSS('.tooltip')).toBeVisible();

      // tooltip is below the anchor
      await expect.poll(() => rects().tooltip.top - rects().anchor.bottom).toBeCloseTo(5);
    });

    it('flips the tooltip when there is no room on the configured side', async () => {
      await render(TooltipTestWrapper, { text: 'short', width: '200px', marginTop: '0px' });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));
      await expect.element(page.getByCSS('.tooltip')).toBeVisible();

      // tooltip is below the anchor
      await expect.poll(() => rects().tooltip.top - rects().anchor.bottom).toBeCloseTo(5);
    });

    it('shifts the tooltip to keep it inside the viewport', async () => {
      await render(TooltipTestWrapper, { text: 'A tooltip that is much wider than its anchor', width: '20px' });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));
      await expect.element(page.getByCSS('.tooltip')).toBeVisible();

      await expect.poll(() => rects().tooltip.left).toBeCloseTo(5);
    });

    it('follows the anchor when it changes size', async () => {
      await render(TooltipTestWrapper, { text: 'short', width: '200px' });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));
      await expect.element(page.getByCSS('.tooltip')).toBeVisible();
      await expect.poll(() => horizontalCenterOffset()).toBeCloseTo(0);

      // Resize through the DOM, because `rerender()` would recreate the
      // tooltip attachment and reposition the tooltip on its own.
      document.querySelector<HTMLElement>('[data-test-anchor]')!.style.width = '300px';
      expect(horizontalCenterOffset()).toBeCloseTo(-50);

      await expect.poll(() => horizontalCenterOffset()).toBeCloseTo(0);
    });
  });

  describe('delay', () => {
    beforeEach(() => vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] }));
    afterEach(() => vi.useRealTimers());

    it('waits for the configured delay before showing the tooltip', async () => {
      await render(TooltipTestWrapper, { text: 'short', width: '500px', delay: 200 });

      // Browser actions advance fake timers while they run, so dispatch the
      // event directly to keep the fake clock under the test's control.
      document.querySelector('[data-test-anchor]')!.dispatchEvent(new MouseEvent('mouseenter'));
      await tick();

      expect(document.querySelector('.tooltip')).toBeNull();

      await vi.advanceTimersByTimeAsync(199);
      await tick();

      expect(document.querySelector('.tooltip')).toBeNull();

      await vi.advanceTimersByTimeAsync(1);
      await tick();

      await expect.element(page.getByCSS('.tooltip')).toBeVisible();
    });
  });

  describe('onlyWhenTruncated', () => {
    it('shows the tooltip when the anchor content is truncated', async () => {
      await render(TooltipTestWrapper, {
        text: 'A very long string that does not fit into the narrow anchor element',
        width: '50px',
        onlyWhenTruncated: true,
      });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));

      await expect.element(page.getByCSS('.tooltip')).toBeVisible();
    });

    it('does not show the tooltip when the anchor content fits', async () => {
      await render(TooltipTestWrapper, { text: 'short', width: '500px', onlyWhenTruncated: true });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));
      await tick();

      expect(document.querySelector('.tooltip')).toBeNull();
    });

    it('checks a separate truncation target when provided', async () => {
      await render(TooltipTestWrapper, {
        text: 'A very long string that does not fit into the narrow target element',
        width: '500px',
        truncationTargetWidth: '50px',
        onlyWhenTruncated: true,
      });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));

      await expect.element(page.getByCSS('.tooltip')).toBeVisible();
    });
  });
});
