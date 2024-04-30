import { test as base } from '@playwright/test';
import { MiragePage } from './fixtures/mirage';
import { PercyPage } from './fixtures/percy';

export interface AppFixtures {
  mirage: MiragePage;
  percy: PercyPage;
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
});

export { expect } from '@playwright/test';
