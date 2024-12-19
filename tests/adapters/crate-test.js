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

  test('findRecord requests do not include versions by default', async function (assert) {
    let _foo = this.server.create('crate', { name: 'foo' });
    let version = this.server.create('version', { crate: _foo });

    let store = this.owner.lookup('service:store');

    let foo = await store.findRecord('crate', 'foo');
    assert.strictEqual(foo?.name, 'foo');

    // versions should not be loaded yet
    let versionsRef = foo.hasMany('versions');
    assert.deepEqual(versionsRef.ids(), []);

    await versionsRef.load();
    assert.deepEqual(versionsRef.ids(), [version.id]);
  });

  test('findHasMany `versions` with `release_tracks` meta', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '0.0.1' });
    this.server.create('version', { crate, num: '0.1.0' });
    this.server.create('version', { crate, num: '1.0.0' });

    let store = this.owner.lookup('service:store');

    let foo = await store.findRecord('crate', 'foo');
    assert.strictEqual(foo?.name, 'foo');
    assert.strictEqual(foo?.versions_map?.release_tracks, undefined);

    // load `versions` without `release_tracks` meta included
    let versionsRef = foo.hasMany('versions');
    await versionsRef.load();
    assert.strictEqual(foo?.versions_meta?.release_tracks, undefined);

    // reload `versions` with `release_tracks` meta included
    let resp = await versionsRef.reload({ adapterOptions: { withReleaseTracks: true } });
    let { meta } = resp;
    assert.deepEqual(foo?.versions_meta?.release_tracks, meta.release_tracks);
  });
});
