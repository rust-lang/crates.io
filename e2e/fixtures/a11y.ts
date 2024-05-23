import { Page } from '@playwright/test';
import { AxeBuilder } from '@axe-core/playwright';
import { expect } from '@playwright/test';

// Ref: https://playwright.dev/docs/accessibility-testing#scanning-an-entire-page
export class A11yPage {
  builder: AxeBuilder;

  constructor(public readonly page: Page) {
    this.page = page;
    this.builder = new AxeBuilder({ page });
  }

  options(...options: Parameters<AxeBuilder['options']>) {
    this.builder = this.builder.options(...options);
    return this;
  }

  async audit() {
    const result = await this.builder.analyze();
    this._check(result);
  }

  async auditWith(options?: Parameters<AxeBuilder['options']>[0]) {
    let builder = new AxeBuilder({ page: this.page });
    if (options) {
      builder = builder.options(options);
    }
    const result = await builder.analyze();
    this._check(result);
  }

  private _check(result: Awaited<ReturnType<AxeBuilder['analyze']>>) {
    expect(result.violations).toEqual([]);
  }
}
