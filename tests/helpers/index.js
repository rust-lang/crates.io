import { setupApplicationTest as upstreamSetupApplicationTest } from 'ember-qunit';

export { setupTest, setupRenderingTest } from 'ember-qunit';

// see http://emberjs.github.io/rfcs/0637-customizable-test-setups.html
export function setupApplicationTest(hooks, options) {
  upstreamSetupApplicationTest(hooks, options);
}
