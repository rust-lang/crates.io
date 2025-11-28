import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0/unyank', { method: 'PUT' });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/unyank', { method: 'PUT' });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns 404 for unknown versions', async function () {
  await db.crate.create({ name: 'foo' });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/unyank', { method: 'PUT' });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('unyanks the version', async function () {
  let crate = await db.crate.create({ name: 'foo' });
  let version = await db.version.create({ crate, num: '1.0.0', yanked: true, yank_message: 'some reason' });
  assert.strictEqual(version.yanked, true);
  assert.strictEqual(version.yank_message, 'some reason');

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/unyank', { method: 'PUT' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });

  version = db.version.findFirst(q => q.where({ id: version.id }));
  assert.strictEqual(version.yanked, false);
  assert.strictEqual(version.yank_message, null);
});
