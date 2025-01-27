import { test as base } from '@playwright/test';
import { FakeTimers, FakeTimersOptions } from './fixtures/fake-timers';
import { MiragePage } from './fixtures/mirage';
import { PercyPage } from './fixtures/percy';
import { A11yPage } from './fixtures/a11y';
import { EmberPage, EmberPageOptions } from './fixtures/ember';
import axeConfig from '@/tests/axe-config';

export type AppOptions = {
  clockOptions: FakeTimersOptions;
  emberOptions: EmberPageOptions;
};
export interface AppFixtures {
  clock: FakeTimers;
  mirage: MiragePage;
  ember: EmberPage;
  percy: PercyPage;
  a11y: A11yPage;
}

export const test = base.extend<AppOptions & AppFixtures>({
  clockOptions: [{ now: '2017-11-20T12:00:00', shouldAdvanceTime: true }, { option: true }],
  emberOptions: [{ setTesting: true, mockSentry: true }, { option: true }],
  clock: [
    async ({ page, clockOptions }, use) => {
      let clock = new FakeTimers(page);
      if (clockOptions != null) {
        await clock.setup(clockOptions);
      }
      await use(clock);
    },
    { auto: true, scope: 'test' },
  ],
  mirage: [
    async ({ page }, use) => {
      let mirage = new MiragePage(page);
      await mirage.setup();
      await use(mirage);
    },
    { scope: 'test' },
  ],
  ember: [
    async ({ page, emberOptions }, use) => {
      let ember = new EmberPage(page);
      await ember.setup(emberOptions);
      await use(ember);
    },
    { auto: true, scope: 'test' },
  ],
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
