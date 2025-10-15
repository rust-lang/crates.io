import axeConfig from '@/tests/axe-config';
import { db, handlers } from '@crates-io/msw';
import { defineNetworkFixture, NetworkFixture } from '@msw/playwright';
import { test as base } from '@playwright/test';
import * as pwFakeTimers from '@sinonjs/fake-timers';

import { A11yPage } from './fixtures/a11y';
import { EmberPage, EmberPageOptions } from './fixtures/ember';
import { FakeTimers, FakeTimersOptions } from './fixtures/fake-timers';
import { PercyPage } from './fixtures/percy';

const TEST_APP = process.env.TEST_APP ?? 'ember';

export type AppOptions = {
  clockOptions: FakeTimersOptions;
  emberOptions: EmberPageOptions;
};
export interface AppFixtures {
  clock: FakeTimers;
  msw: {
    worker: NetworkFixture;
    db: typeof db;
    authenticateAs: (user: any) => Promise<void>;
  };
  ember: EmberPage;
  percy: PercyPage;
  a11y: A11yPage;
}

export const test = base.extend<AppOptions & AppFixtures>({
  clockOptions: [{ now: '2017-11-20T12:00:00', shouldAdvanceTime: true }, { option: true }],
  emberOptions: [{ setTesting: true, mockSentry: true }, { option: true }],
  clock: [
    async ({ page, clockOptions }, use) => {
      let now = clockOptions.now;
      if (typeof now === 'string') {
        now = Date.parse(now);
      }

      let pwClock = pwFakeTimers.install({
        ...clockOptions,
        now,
        toFake: ['Date'],
      });

      let clock = new FakeTimers(page);
      if (clockOptions != null) {
        await clock.setup(clockOptions);
      }
      await use(clock);
      pwClock?.uninstall();
    },
    { auto: true, scope: 'test' },
  ],
  msw: [
    async ({ context, page }, use) => {
      const worker = defineNetworkFixture({
        context,
        handlers,
        // Without this, requests for `foo.json` cannot be intercepted, which causes some tests to fail.
        skipAssetRequests: false,
      });
      await worker.enable();

      const authenticateAs = async function (user) {
        await db.mswSession.create({ user });
        await page.addInitScript("globalThis.localStorage.setItem('isLoggedIn', '1')");
      };

      await use({ worker, db, authenticateAs });
      await db.reset();
      await worker.disable();
    },
    { auto: true, scope: 'test' },
  ],
  ember: [
    async ({ page, emberOptions }, use) => {
      let ember = new EmberPage(page);
      await ember.setup(emberOptions);
      await use(ember);
    },
    { auto: TEST_APP === 'ember', scope: 'test' },
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
