import { module, test } from 'qunit';

import { setupMirage, setupTest } from 'crates-io/tests/helpers';

module('Adapter | crate', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('findRecord requests are coalesced', async function (assert) {
    let _foo = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate: _foo });
    let _bar = this.server.create('crate', { name: 'bar' });
    this.server.create('version', { crate: _bar });

    // if request coalescing works correctly, then this regular API endpoint
    // should not be hit in this case
    this.server.get('/api/v1/crates/:crate_name', {}, 500);

    let store = this.owner.lookup('service:store');

    let [foo, bar] = await Promise.all([store.findRecord('crate', 'foo'), store.findRecord('crate', 'bar')]);
    assert.strictEqual(foo?.name, 'foo');
    assert.strictEqual(bar?.name, 'bar');
  });
});
