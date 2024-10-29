import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../../helpers';
import setupMirage from '../../../../helpers/setup-mirage';

module('Mirage | DELETE /api/v1/crates/:crateId/yank', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 403 if unauthenticated', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/1.0.0/yank', { method: 'DELETE' });
    assert.strictEqual(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });

  test('returns 404 for unknown crates', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/foo/1.0.0/yank', { method: 'DELETE' });
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns 404 for unknown versions', async function (assert) {
    this.server.create('crate', { name: 'foo' });

    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/foo/1.0.0/yank', { method: 'DELETE' });
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('yanks the version', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    let version = this.server.create('version', { crate, num: '1.0.0', yanked: false });
    assert.false(version.yanked);

    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/foo/1.0.0/yank', { method: 'DELETE' });
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), { ok: true });

    user.reload();
    assert.true(version.yanked);
  });
});
