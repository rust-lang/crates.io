import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/following');
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/following');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns true if the authenticated user follows the crate', async function () {
  let crate = db.crate.create({ name: 'rand' });

  let user = db.user.create({ followedCrates: [crate] });
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/rand/following');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { following: true });
});

test('returns false if the authenticated user is not following the crate', async function () {
  db.crate.create({ name: 'rand' });

  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/rand/following');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { following: false });
});
