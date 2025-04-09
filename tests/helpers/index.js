import { setupApplicationTest as upstreamSetupApplicationTest } from 'ember-qunit';

import { setupSentryMock } from './sentry';
import setupMSW from './setup-msw';

export { setupTest, setupRenderingTest } from 'ember-qunit';

// see http://emberjs.github.io/rfcs/0637-customizable-test-setups.html
export function setupApplicationTest(hooks, options = {}) {
  upstreamSetupApplicationTest(hooks, options);

  setupMSW(hooks);
  setupSentryMock(hooks);
  setupAppTestDataAttr(hooks);
}

function setupAppTestDataAttr(hooks) {
  hooks.beforeEach(function () {
    document.documentElement.dataset.appTest = '';
  });

  hooks.afterEach(function () {
    delete document.documentElement.dataset.appTest;
  });
}
