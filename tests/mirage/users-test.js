import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import fetch from 'fetch';

import setupMirage from '../helpers/setup-mirage';

module('Mirage | Users', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('GET /api/v1/users/:id', function () {
    test('returns 404 for unknown users', async function (assert) {
      let response = await fetch('/api/v1/users/foo');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('returns a user object for known users', async function (assert) {
      let user = this.server.create('user', { name: 'John Doe' });

      let response = await fetch(`/api/v1/users/${user.login}`);
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        user: {
          id: 1,
          avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
          login: 'john-doe',
          name: 'John Doe',
          url: 'https://github.com/john-doe',
        },
      });
    });
  });

  module('PUT /api/v1/users/:id', function () {
    test('updates the user with a new email address', async function (assert) {
      let user = this.server.create('user', { email: 'old@email.com' });
      this.server.create('mirage-session', { user });

      let body = JSON.stringify({ user: { email: 'new@email.com' } });
      let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { ok: true });

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

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'must be logged in to perform that action' }] });

      user.reload();
      assert.strictEqual(user.email, 'old@email.com');
    });

    test('returns 400 when requesting the wrong user id', async function (assert) {
      let user = this.server.create('user', { email: 'old@email.com' });
      this.server.create('mirage-session', { user });

      let body = JSON.stringify({ user: { email: 'new@email.com' } });
      let response = await fetch(`/api/v1/users/wrong-id`, { method: 'PUT', body });
      assert.equal(response.status, 400);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'current user does not match requested user' }] });

      user.reload();
      assert.strictEqual(user.email, 'old@email.com');
    });

    test('returns 400 when sending an invalid payload', async function (assert) {
      let user = this.server.create('user', { email: 'old@email.com' });
      this.server.create('mirage-session', { user });

      let body = JSON.stringify({});
      let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
      assert.equal(response.status, 400);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'invalid json request' }] });

      user.reload();
      assert.strictEqual(user.email, 'old@email.com');
    });

    test('returns 400 when sending an empty email address', async function (assert) {
      let user = this.server.create('user', { email: 'old@email.com' });
      this.server.create('mirage-session', { user });

      let body = JSON.stringify({ user: { email: '' } });
      let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
      assert.equal(response.status, 400);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'empty email rejected' }] });

      user.reload();
      assert.strictEqual(user.email, 'old@email.com');
    });
  });

  module('PUT /api/v1/users/:id/resend', function () {
    test('returns `ok`', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let response = await fetch(`/api/v1/users/${user.id}/resend`, { method: 'PUT' });
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { ok: true });
    });

    test('returns 403 when not logged in', async function (assert) {
      let user = this.server.create('user');

      let response = await fetch(`/api/v1/users/${user.id}/resend`, { method: 'PUT' });
      assert.equal(response.status, 403);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'must be logged in to perform that action' }] });
    });

    test('returns 400 when requesting the wrong user id', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let response = await fetch(`/api/v1/users/wrong-id/resend`, { method: 'PUT' });
      assert.equal(response.status, 400);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'current user does not match requested user' }] });
    });
  });
});
