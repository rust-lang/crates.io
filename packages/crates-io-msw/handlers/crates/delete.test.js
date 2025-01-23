import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo', { method: 'DELETE' });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo', { method: 'DELETE' });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'crate `foo` does not exist' }] });
});

test('deletes crates', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let crate = db.crate.create({ name: 'foo' });
  db.crateOwnership.create({ crate, user });

  let response = await fetch('/api/v1/crates/foo', { method: 'DELETE' });
  assert.strictEqual(response.status, 204);
  assert.deepEqual(await response.text(), '');

  assert.strictEqual(db.crate.findFirst({ where: { name: { equals: 'foo' } } }), null);
});
