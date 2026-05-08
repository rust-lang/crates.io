import percySnapshot from '@percy/playwright';
import { expect, Page, TestInfo } from '@playwright/test';

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
    // Wait for any in-flight loading state to settle before snapshotting,
    // otherwise spinners can leak into the captured image and produce flaky
    // visual diffs.
    await expect(this.page.locator('[data-test-spinner]')).toHaveCount(0);

    await percySnapshot(this.page, this.title(), options);
  }
}
