import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/following');
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/following');
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({ errors: [{ detail: 'Not Found' }] });
});

test('returns true if the authenticated user follows the crate', async function () {
  let crate = await db.crate.create({ name: 'rand' });

  let user = await db.user.create({ followedCrates: [crate] });
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/rand/following');
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ following: true });
});

test('returns false if the authenticated user is not following the crate', async function () {
  await db.crate.create({ name: 'rand' });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/rand/following');
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ following: false });
});
