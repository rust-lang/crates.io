import type { AnchorOptions } from './anchor-to';

import { afterEach, describe, expect, it } from 'vitest';

import { anchorTo } from './anchor-to';

const OPTIONS: AnchorOptions = { placement: 'bottom-start', offset: 0, padding: 5 };

let cleanups: (() => void)[] = [];

afterEach(() => {
  for (let cleanup of cleanups.splice(0)) cleanup();
});

function setup(anchorWidth: string, elementText: string) {
  let anchor = document.createElement('div');
  anchor.style.width = anchorWidth;
  anchor.style.height = '20px';

  let element = document.createElement('div');
  element.style.position = 'absolute';
  element.style.width = 'max-content';
  element.textContent = elementText;

  document.body.append(anchor, element);
  cleanups.push(() => {
    anchor.remove();
    element.remove();
  });

  return { anchor, element };
}

function attach(anchor: HTMLElement, element: HTMLElement, options: AnchorOptions) {
  let cleanup = anchorTo(anchor, options)(element);
  if (cleanup) cleanups.push(cleanup);
}

describe('anchorTo', () => {
  describe('--anchor-width', () => {
    it('exposes the width of the anchor without changing the element', async () => {
      let { anchor, element } = setup('200px', 'short');
      attach(anchor, element, OPTIONS);

      await expect.poll(() => element.style.getPropertyValue('--anchor-width')).toBe('200px');
      expect(element.getBoundingClientRect().width).toBeLessThan(200);
    });

    it('follows the anchor when it resizes', async () => {
      let { anchor, element } = setup('200px', 'short');
      attach(anchor, element, OPTIONS);
      await expect.poll(() => element.style.getPropertyValue('--anchor-width')).toBe('200px');

      anchor.style.width = '300px';

      await expect.poll(() => element.style.getPropertyValue('--anchor-width')).toBe('300px');
    });
  });
});
