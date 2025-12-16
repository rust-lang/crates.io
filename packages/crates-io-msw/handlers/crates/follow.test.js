import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/follow', { method: 'PUT' });
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/follow', { method: 'PUT' });
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({ errors: [{ detail: 'Not Found' }] });
});

test('makes the authenticated user follow the crate', async function () {
  let crate = await db.crate.create({ name: 'rand' });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  expect(user.followedCrates).toEqual([]);

  let response = await fetch('/api/v1/crates/rand/follow', { method: 'PUT' });
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ ok: true });

  user = db.user.findFirst(q => q.where({ id: user.id }));
  expect(user.followedCrates.length).toBe(1);
  expect(user.followedCrates[0].name).toBe(crate.name);
});
