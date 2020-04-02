import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from '../helpers/setup-mirage';
import fetch from 'fetch';
import timekeeper from 'timekeeper';

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

    test('returns a list of `owned_crates`', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let [crate1, , crate3] = this.server.createList('crate', 3);

      this.server.create('crate-ownership', { crate: crate1, user });
      this.server.create('crate-ownership', { crate: crate3, user });

      let response = await fetch('/api/v1/me');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload.owned_crates, [
        { id: 'crate-0', name: 'crate-0', email_notifications: true },
        { id: 'crate-2', name: 'crate-2', email_notifications: true },
      ]);
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

  module('GET /api/v1/me/tokens', function () {
    test('returns the list of API token for the authenticated `user`', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      this.server.create('api-token', { user, createdAt: '2017-11-19T12:59:22Z' });
      this.server.create('api-token', { user, createdAt: '2017-11-19T13:59:22Z' });
      this.server.create('api-token', { user, createdAt: '2017-11-19T14:59:22Z' });

      let response = await fetch('/api/v1/me/tokens');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        api_tokens: [
          {
            id: 3,
            created_at: '2017-11-19T14:59:22.000Z',
            last_used_at: null,
            name: 'API Token 3',
          },
          {
            id: 2,
            created_at: '2017-11-19T13:59:22.000Z',
            last_used_at: null,
            name: 'API Token 2',
          },
          {
            id: 1,
            created_at: '2017-11-19T12:59:22.000Z',
            last_used_at: null,
            name: 'API Token 1',
          },
        ],
      });
    });

    test('empty list case', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let response = await fetch('/api/v1/me/tokens');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { api_tokens: [] });
    });

    test('returns an error if unauthenticated', async function (assert) {
      let response = await fetch('/api/v1/me/tokens');
      assert.equal(response.status, 403);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        errors: [{ detail: 'must be logged in to perform that action' }],
      });
    });
  });

  module('PUT /api/v1/me/tokens', function () {
    test('creates a new API token', async function (assert) {
      timekeeper.freeze(new Date('2017-11-20T11:23:45Z'));

      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let body = JSON.stringify({ api_token: { name: 'foooo' } });
      let response = await fetch('/api/v1/me/tokens', { method: 'PUT', body });
      assert.equal(response.status, 200);

      let token = this.server.schema.apiTokens.all().models[0];
      assert.ok(token);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        api_token: {
          id: 1,
          created_at: '2017-11-20T11:23:45.000Z',
          last_used_at: null,
          name: 'foooo',
          revoked: false,
          token: token.token,
        },
      });
    });

    test('returns an error if unauthenticated', async function (assert) {
      let body = JSON.stringify({ api_token: {} });
      let response = await fetch('/api/v1/me/tokens', { method: 'PUT', body });
      assert.equal(response.status, 403);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        errors: [{ detail: 'must be logged in to perform that action' }],
      });
    });
  });

  module('DELETE /api/v1/me/tokens/:tokenId', function () {
    test('revokes an API token', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let token = this.server.create('api-token', { user });

      let response = await fetch(`/api/v1/me/tokens/${token.id}`, { method: 'DELETE' });
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {});

      let tokens = this.server.schema.apiTokens.all().models;
      assert.equal(tokens.length, 0);
    });

    test('returns an error if unauthenticated', async function (assert) {
      let user = this.server.create('user');
      let token = this.server.create('api-token', { user });

      let response = await fetch(`/api/v1/me/tokens/${token.id}`, { method: 'DELETE' });
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
