import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns the `user` resource including the private fields', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/me');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    user: {
      id: 1,
      avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
      email: 'user-1@crates.io',
      email_verification_sent: true,
      email_verified: true,
      is_admin: false,
      login: 'user-1',
      name: 'User 1',
      publish_notifications: true,
      url: 'https://github.com/user-1',
    },
    owned_crates: [],
  });
});

test('returns a list of `owned_crates`', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let [crate1, , crate3] = Array.from({ length: 3 }, () => db.crate.create());

  db.crateOwnership.create({ crate: crate1, user });
  db.crateOwnership.create({ crate: crate3, user });

  let response = await fetch('/api/v1/me');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.deepEqual(responsePayload.owned_crates, [
    { id: crate1.id, name: 'crate-1', email_notifications: true },
    { id: crate3.id, name: 'crate-3', email_notifications: true },
  ]);
});

test('returns an error if unauthenticated', async function () {
  db.user.create();

  let response = await fetch('/api/v1/me');
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
