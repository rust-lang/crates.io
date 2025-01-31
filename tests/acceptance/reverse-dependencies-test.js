import { click, currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | /crates/:crate_id/reverse_dependencies', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  function prepare({ db }) {
    let foo = db.crate.create({ name: 'foo' });
    db.version.create({ crate: foo });

    let bar = db.crate.create({ name: 'bar' });
    let barVersion = db.version.create({ crate: bar });

    let baz = db.crate.create({ name: 'baz' });
    let bazVersion = db.version.create({ crate: baz });

    db.dependency.create({ crate: foo, version: barVersion });
    db.dependency.create({ crate: foo, version: bazVersion });

    return { foo, bar, baz };
  }

  test('shows a list of crates depending on the selected crate', async function (assert) {
    let { foo, bar, baz } = prepare(this);

    await visit(`/crates/${foo.name}/reverse_dependencies`);
    assert.strictEqual(currentURL(), `/crates/${foo.name}/reverse_dependencies`);
    assert.dom('[data-test-row]').exists({ count: 2 });
    assert.dom('[data-test-row="0"] [data-test-crate-name]').hasText(baz.name);
    assert.dom('[data-test-row="0"] [data-test-description]').hasText(baz.description);
    assert.dom('[data-test-row="1"] [data-test-crate-name]').hasText(bar.name);
    assert.dom('[data-test-row="1"] [data-test-description]').hasText(bar.description);
  });

  test('supports pagination', async function (assert) {
    let { foo } = prepare(this);

    for (let i = 0; i < 20; i++) {
      let crate = this.db.crate.create();
      let version = this.db.version.create({ crate });
      this.db.dependency.create({ crate: foo, version });
    }

    await visit(`/crates/${foo.name}/reverse_dependencies`);
    assert.strictEqual(currentURL(), `/crates/${foo.name}/reverse_dependencies`);
    assert.dom('[data-test-row]').exists({ count: 10 });
    assert.dom('[data-test-current-rows]').hasText('1-10');
    assert.dom('[data-test-total-rows]').hasText('22');

    await click('[data-test-pagination-next]');
    assert.strictEqual(currentURL(), `/crates/${foo.name}/reverse_dependencies?page=2`);
    assert.dom('[data-test-row]').exists({ count: 10 });
    assert.dom('[data-test-current-rows]').hasText('11-20');
    assert.dom('[data-test-total-rows]').hasText('22');

    await click('[data-test-pagination-next]');
    assert.strictEqual(currentURL(), `/crates/${foo.name}/reverse_dependencies?page=3`);
    assert.dom('[data-test-row]').exists({ count: 2 });
    assert.dom('[data-test-current-rows]').hasText('21-22');
    assert.dom('[data-test-total-rows]').hasText('22');
  });

  test('shows a generic error if the server is broken', async function (assert) {
    let { foo } = prepare(this);

    let error = HttpResponse.json({}, { status: 500 });
    this.worker.use(http.get('/api/v1/crates/:crate_id/reverse_dependencies', () => error));

    await visit(`/crates/${foo.name}/reverse_dependencies`);
    assert.strictEqual(currentURL(), '/');
    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Could not load reverse dependencies for the "foo" crate');
  });

  test('shows a detailed error if available', async function (assert) {
    let { foo } = prepare(this);

    let error = HttpResponse.json({ errors: [{ detail: 'cannot request more than 100 items' }] }, { status: 400 });
    this.worker.use(http.get('/api/v1/crates/:crate_id/reverse_dependencies', () => error));

    await visit(`/crates/${foo.name}/reverse_dependencies`);
    assert.strictEqual(currentURL(), '/');
    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Could not load reverse dependencies for the "foo" crate: cannot request more than 100 items');
  });
});
