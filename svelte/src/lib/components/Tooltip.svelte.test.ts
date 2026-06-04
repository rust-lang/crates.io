import { tick } from 'svelte';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import TooltipTestWrapper from './TooltipTestWrapper.svelte';

describe('Tooltip', () => {
  it('shows the tooltip on hover', async () => {
    render(TooltipTestWrapper, { text: 'short', width: '500px' });

    await userEvent.hover(page.getByCSS('[data-test-anchor]'));

    await expect.element(page.getByCSS('.tooltip')).toBeVisible();
  });

  describe('delay', () => {
    it('waits for the configured delay before showing the tooltip', async () => {
      render(TooltipTestWrapper, { text: 'short', width: '500px', delay: 200 });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));
      await tick();

      expect(document.querySelector('.tooltip')).toBeNull();

      await expect.element(page.getByCSS('.tooltip')).toBeVisible();
    });
  });

  describe('onlyWhenTruncated', () => {
    it('shows the tooltip when the anchor content is truncated', async () => {
      render(TooltipTestWrapper, {
        text: 'A very long string that does not fit into the narrow anchor element',
        width: '50px',
        onlyWhenTruncated: true,
      });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));

      await expect.element(page.getByCSS('.tooltip')).toBeVisible();
    });

    it('does not show the tooltip when the anchor content fits', async () => {
      render(TooltipTestWrapper, { text: 'short', width: '500px', onlyWhenTruncated: true });

      await userEvent.hover(page.getByCSS('[data-test-anchor]'));
      await tick();

      expect(document.querySelector('.tooltip')).toBeNull();
    });
  });
});
