import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | PUT /api/v1/users/:id', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('updates the user with a new email address', async function (assert) {
    let user = this.server.create('user', { email: 'old@email.com' });
    this.server.create('mirage-session', { user });

    let body = JSON.stringify({ user: { email: 'new@email.com' } });
    let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { ok: true });

    user.reload();
    assert.strictEqual(user.email, 'new@email.com');
    assert.strictEqual(user.emailVerified, false);
    assert.strictEqual(user.emailVerificationToken, 'secret123');
  });

  test('returns 403 when not logged in', async function (assert) {
    let user = this.server.create('user', { email: 'old@email.com' });

    let body = JSON.stringify({ user: { email: 'new@email.com' } });
    let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
    assert.equal(response.status, 403);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'must be logged in to perform that action' }] });

    user.reload();
    assert.strictEqual(user.email, 'old@email.com');
  });

  test('returns 400 when requesting the wrong user id', async function (assert) {
    let user = this.server.create('user', { email: 'old@email.com' });
    this.server.create('mirage-session', { user });

    let body = JSON.stringify({ user: { email: 'new@email.com' } });
    let response = await fetch(`/api/v1/users/wrong-id`, { method: 'PUT', body });
    assert.equal(response.status, 400);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'current user does not match requested user' }] });

    user.reload();
    assert.strictEqual(user.email, 'old@email.com');
  });

  test('returns 400 when sending an invalid payload', async function (assert) {
    let user = this.server.create('user', { email: 'old@email.com' });
    this.server.create('mirage-session', { user });

    let body = JSON.stringify({});
    let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
    assert.equal(response.status, 400);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'invalid json request' }] });

    user.reload();
    assert.strictEqual(user.email, 'old@email.com');
  });

  test('returns 400 when sending an empty email address', async function (assert) {
    let user = this.server.create('user', { email: 'old@email.com' });
    this.server.create('mirage-session', { user });

    let body = JSON.stringify({ user: { email: '' } });
    let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
    assert.equal(response.status, 400);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'empty email rejected' }] });

    user.reload();
    assert.strictEqual(user.email, 'old@email.com');
  });
});
