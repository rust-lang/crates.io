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
});
