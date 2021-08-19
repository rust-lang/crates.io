import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | PUT /api/v1/users/:id/resend', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns `ok`', async function (assert) {
    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    let response = await fetch(`/api/v1/users/${user.id}/resend`, { method: 'PUT' });
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { ok: true });
  });

  test('returns 403 when not logged in', async function (assert) {
    let user = this.server.create('user');

    let response = await fetch(`/api/v1/users/${user.id}/resend`, { method: 'PUT' });
    assert.equal(response.status, 403);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'must be logged in to perform that action' }] });
  });

  test('returns 400 when requesting the wrong user id', async function (assert) {
    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    let response = await fetch(`/api/v1/users/wrong-id/resend`, { method: 'PUT' });
    assert.equal(response.status, 400);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'current user does not match requested user' }] });
  });
});
