import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns the `user` resource including the private fields', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/me');
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
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
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let [crate1, , crate3] = await Promise.all(Array.from({ length: 3 }, () => db.crate.create()));

  await db.crateOwnership.create({ crate: crate1, user });
  await db.crateOwnership.create({ crate: crate3, user });

  let response = await fetch('/api/v1/me');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.owned_crates).toEqual([
    { id: crate1.id, name: 'crate-1', email_notifications: true },
    { id: crate3.id, name: 'crate-3', email_notifications: true },
  ]);
});

test('returns an error if unauthenticated', async function () {
  await db.user.create();

  let response = await fetch('/api/v1/me');
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
