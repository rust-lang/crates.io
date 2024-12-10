import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | DELETE /api/v1/crates/:name', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 403 if unauthenticated', async function (assert) {
    let response = await fetch('/api/v1/crates/foo', { method: 'DELETE' });
    assert.strictEqual(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });

  test('returns 404 for unknown crates', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/foo', { method: 'DELETE' });
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'crate `foo` does not exist' }] });
  });

  test('deletes crates', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('crate-ownership', { crate, user });

    let response = await fetch('/api/v1/crates/foo', { method: 'DELETE' });
    assert.strictEqual(response.status, 204);
    assert.deepEqual(await response.text(), '');

    assert.strictEqual(this.server.schema.crates.findBy({ name: 'foo' }), null);
  });
});
