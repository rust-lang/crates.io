import { default as percySnapshot_ } from '@percy/playwright';
import { Page, TestInfo, test as base } from '@playwright/test';
import { MiragePage } from './fixtures/mirage';

export const percySnapshot = (page: Page, testInfo: TestInfo, option?: Parameters<typeof percySnapshot_>[2]) => {
  // Snapshot with a title that mimics @percy/ember
  const titlePath = testInfo.titlePath.length > 2 ? testInfo.titlePath.slice(1) : testInfo.titlePath;
  return percySnapshot_(page, titlePath.join(' | '), option);
};

export interface AppFixtures {
  mirage: MiragePage;
}

export const test = base.extend<AppFixtures>({
  mirage: async ({ page }, use) => {
    let mirage = new MiragePage(page);
    await mirage.setup();
    await use(mirage);
  },
});

export { expect } from '@playwright/test';
