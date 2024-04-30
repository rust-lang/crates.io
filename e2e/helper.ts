import { test as base } from '@playwright/test';
import { MiragePage } from './fixtures/mirage';
import { PercyPage } from './fixtures/percy';
import { A11yPage } from './fixtures/a11y';
import axeConfig from '@/tests/axe-config';

export interface AppFixtures {
  mirage: MiragePage;
  percy: PercyPage;
  a11y: A11yPage;
}

export const test = base.extend<AppFixtures>({
  mirage: async ({ page }, use) => {
    let mirage = new MiragePage(page);
    await mirage.setup();
    await use(mirage);
  },
  percy: async ({ page }, use, testInfo) => {
    let percy = new PercyPage(page, testInfo);
    await use(percy);
  },
  a11y: async ({ page }, use) => {
    let a11y = new A11yPage(page);
    a11y = a11y.options(axeConfig);
    await use(a11y);
  },
});

export { expect } from '@playwright/test';
