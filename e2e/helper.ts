import { default as percySnapshot_ } from '@percy/playwright';
import { Page, TestInfo } from '@playwright/test';

export const percySnapshot = (page: Page, testInfo: TestInfo, option?: Parameters<typeof percySnapshot_>[2]) => {
  // Snapshot with a title that mimics @percy/ember
  const titlePath = testInfo.titlePath.length > 2 ? testInfo.titlePath.slice(1) : testInfo.titlePath;
  return percySnapshot_(page, titlePath.join(' | '), option);
};
