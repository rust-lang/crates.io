import { Page, TestInfo } from '@playwright/test';
import { default as percySnapshot } from '@percy/playwright';

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
    const paths = this.testInfo.titlePath.slice(1);
    return paths.join(' | ');
  }

  async snapshot(options?: Parameters<typeof percySnapshot>[2]) {
    await percySnapshot(this.page, this.title(), options);
  }
}
