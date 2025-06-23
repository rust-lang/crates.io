import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0/rebuild_docs', { method: 'POST' });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/rebuild_docs', { method: 'POST' });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns 404 for unknown versions', async function () {
  db.crate.create({ name: 'foo' });

  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/rebuild_docs', { method: 'POST' });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('triggers a rebuild for the crate documentation', async function () {
  let crate = db.crate.create({ name: 'foo' });
  db.version.create({ crate, num: '1.0.0' });

  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/rebuild_docs', { method: 'POST' });
  assert.strictEqual(response.status, 201);
  assert.deepEqual(await response.text(), '');
});
