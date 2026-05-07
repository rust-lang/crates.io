import percySnapshot from '@percy/playwright';
import { Page, TestInfo } from '@playwright/test';

export class PercyPage {
  constructor(
    public readonly page: Page,
    public readonly testInfo: TestInfo,
  ) {
    this.page = page;
    this.testInfo = testInfo;
  }

  // This implementation maintains the title format used by @percy/ember
  private title(): string {
    // Skip the filename
    return this.testInfo.titlePath.slice(1).join(' | ');
  }

  async snapshot(options?: Parameters<typeof percySnapshot>[2]) {
    await percySnapshot(this.page, this.title(), options);
  }
}
