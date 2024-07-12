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
    // Add an "e2e" prefix to differentiate the snapshots from the QUnit tests.
    // This address the visual changes caused by the font not loading in QUnit tests (#9052).
    return ['e2e'].concat(paths).join(' | ');
  }

  async snapshot(options?: Parameters<typeof percySnapshot>[2]) {
    await percySnapshot(this.page, this.title(), options);
  }
}
