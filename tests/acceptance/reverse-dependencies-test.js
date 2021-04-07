import { click, currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | /crates/:crate_id/reverse_dependencies', function (hooks) {
  setupApplicationTest(hooks);

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
    assert.dom('[data-test-row="0"] [data-test-description]').hasText(bar.description);
    assert.dom('[data-test-row="1"] [data-test-crate-name]').hasText(baz.name);
    assert.dom('[data-test-row="1"] [data-test-description]').hasText(baz.description);
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

  test('shows an error if the server is broken', async function (assert) {
    let { foo } = prepare(this);

    this.server.get('/api/v1/crates/:crate_id/reverse_dependencies', {}, 500);

    await visit(`/crates/${foo.name}/reverse_dependencies`);
    assert.equal(currentURL(), '/');
    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Could not load reverse dependencies for the "foo" crate');
  });

  test('shows an error if the server is broken', async function (assert) {
    let { foo } = prepare(this);

    let payload = { errors: [{ detail: 'cannot request more than 100 items' }] };
    this.server.get('/api/v1/crates/:crate_id/reverse_dependencies', payload, 400);

    await visit(`/crates/${foo.name}/reverse_dependencies`);
    assert.equal(currentURL(), '/');
    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Could not load reverse dependencies for the "foo" crate: cannot request more than 100 items');
  });
});
