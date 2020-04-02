import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from '../helpers/setup-mirage';
import fetch from 'fetch';

module('Mirage | /me', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('GET /api/v1/me', function () {
    test('returns the `user` resource including the private fields', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let response = await fetch('/api/v1/me');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        user: {
          id: 1,
          avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
          email: 'user-1@crates.io',
          email_verification_sent: true,
          email_verified: true,
          login: 'user-1',
          name: 'User 1',
          url: 'https://github.com/user-1',
        },
        owned_crates: [],
      });
    });

    test('returns an error if unauthenticated', async function (assert) {
      this.server.create('user');

      let response = await fetch('/api/v1/me');
      assert.equal(response.status, 403);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        errors: [{ detail: 'must be logged in to perform that action' }],
      });
    });
  });

  module('GET /api/v1/confirm/:token', function () {
    test('returns `ok: true` for a known token (unauthenticated)', async function (assert) {
      let user = this.server.create('user', { emailVerificationToken: 'foo' });
      assert.strictEqual(user.emailVerified, false);

      let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { ok: true });

      user.reload();
      assert.strictEqual(user.emailVerified, true);
    });

    test('returns `ok: true` for a known token (authenticated)', async function (assert) {
      let user = this.server.create('user', { emailVerificationToken: 'foo' });
      assert.strictEqual(user.emailVerified, false);

      this.server.create('mirage-session', { user });

      let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { ok: true });

      user.reload();
      assert.strictEqual(user.emailVerified, true);
    });

    test('returns an error for unknown tokens', async function (assert) {
      let response = await fetch('/api/v1/confirm/unknown', { method: 'PUT' });
      assert.equal(response.status, 400);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        errors: [{ detail: 'Email belonging to token not found.' }],
      });
    });
  });
});
