import { setupApplicationTest as upstreamSetupApplicationTest } from 'ember-qunit';

import { setupSentryMock } from './sentry';
import setupMirage from './setup-mirage';

export { setupTest, setupRenderingTest } from 'ember-qunit';

// see http://emberjs.github.io/rfcs/0637-customizable-test-setups.html
export function setupApplicationTest(hooks, options) {
  upstreamSetupApplicationTest(hooks, options);
  setupMirage(hooks);
  setupSentryMock(hooks);
  setupAppTestBodyClass(hooks);
}

function setupAppTestBodyClass(hooks) {
  const ID = 'app-test';

  hooks.beforeEach(function () {
    this.bodyClass = this.owner.lookup('service:body-class');
    this.bodyClass.register(ID, ['app-test']);
  });

  hooks.afterEach(function () {
    this.bodyClass.deregister(ID);
  });
}
