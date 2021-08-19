import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:crateId/following', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 403 if unauthenticated', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/following');
    assert.equal(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });

  test('returns 404 for unknown crates', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/foo/following');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns true if the authenticated user follows the crate', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });

    let user = this.server.create('user', { followedCrates: [crate] });
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/rand/following');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { following: true });
  });

  test('returns false if the authenticated user is not following the crate', async function (assert) {
    this.server.create('crate', { name: 'rand' });

    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/rand/following');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { following: false });
  });
});
