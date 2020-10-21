import { click, currentURL, visit } from '@ember/test-helpers';
import { setupApplicationTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from '../helpers/setup-mirage';

module('Acceptance | /crates/:crate_id/reverse_dependencies', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  function prepare({ server }) {
    let foo = server.create('crate', { name: 'foo' });
    server.create('version', { crate: foo });

    let bar = server.create('crate', { name: 'bar' });
    server.create('version', { crate: bar });

    let baz = server.create('crate', { name: 'baz' });
    server.create('version', { crate: baz });

    server.create('dependency', { crate: foo, version: bar.versions.models[0] });
    server.create('dependency', { crate: foo, version: baz.versions.models[0] });

    return { foo, bar, baz };
  }

  test('shows a list of crates depending on the selected crate', async function (assert) {
    let { foo, bar, baz } = prepare(this);

    await visit(`/crates/${foo.name}/reverse_dependencies`);
    assert.equal(currentURL(), `/crates/${foo.name}/reverse_dependencies`);
    assert.dom('[data-test-row]').exists({ count: 2 });
    assert.dom('[data-test-row="0"] [data-test-crate-name]').hasText(bar.name);
    assert.dom('[data-test-row="1"] [data-test-crate-name]').hasText(baz.name);
  });

  test('supports pagination', async function (assert) {
    let { foo } = prepare(this);

    for (let i = 0; i < 20; i++) {
      let crate = this.server.create('crate');
      let version = this.server.create('version', { crate });
      this.server.create('dependency', { crate: foo, version });
    }

    await visit(`/crates/${foo.name}/reverse_dependencies`);
    assert.equal(currentURL(), `/crates/${foo.name}/reverse_dependencies`);
    assert.dom('[data-test-row]').exists({ count: 10 });
    assert.dom('[data-test-current-rows]').hasText('1-10');
    assert.dom('[data-test-total-rows]').hasText('22');

    await click('[data-test-pagination-next]');
    assert.equal(currentURL(), `/crates/${foo.name}/reverse_dependencies?page=2`);
    assert.dom('[data-test-row]').exists({ count: 10 });
    assert.dom('[data-test-current-rows]').hasText('11-20');
    assert.dom('[data-test-total-rows]').hasText('22');

    await click('[data-test-pagination-next]');
    assert.equal(currentURL(), `/crates/${foo.name}/reverse_dependencies?page=3`);
    assert.dom('[data-test-row]').exists({ count: 2 });
    assert.dom('[data-test-current-rows]').hasText('21-22');
    assert.dom('[data-test-total-rows]').hasText('22');
  });
});
