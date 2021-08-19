import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/me', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns the `user` resource including the private fields', async function (assert) {
    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    let response = await fetch('/api/v1/me');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
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
      { id: crate1.id, name: 'crate-0', email_notifications: true },
      { id: crate3.id, name: 'crate-2', email_notifications: true },
    ]);
  });

  test('returns an error if unauthenticated', async function (assert) {
    this.server.create('user');

    let response = await fetch('/api/v1/me');
    assert.equal(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });
});
