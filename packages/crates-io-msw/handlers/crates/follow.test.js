import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/follow', { method: 'PUT' });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/follow', { method: 'PUT' });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('makes the authenticated user follow the crate', async function () {
  let crate = await db.crate.create({ name: 'rand' });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  assert.deepEqual(user.followedCrates, []);

  let response = await fetch('/api/v1/crates/rand/follow', { method: 'PUT' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });

  user = db.user.findFirst(q => q.where({ id: user.id }));
  assert.equal(user.followedCrates.length, 1);
  assert.equal(user.followedCrates[0].name, crate.name);
});
