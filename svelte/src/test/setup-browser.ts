import type { Locator } from 'vitest/browser';

import { locators } from 'vitest/browser';

declare module 'vitest/browser' {
  interface LocatorSelectors {
    getByCSS(css: string): Locator;
  }
}

locators.extend({
  getByCSS(css: string) {
    return `css=${css}`;
  },
});
