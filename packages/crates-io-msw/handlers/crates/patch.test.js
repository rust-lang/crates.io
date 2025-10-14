import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ crate: { trustpub_only: true } }),
  });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ crate: { trustpub_only: true } }),
  });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'crate `foo` does not exist' }] });
});

test('updates trustpub_only flag', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let crate = await db.crate.create({ name: 'foo', trustpubOnly: false });
  assert.strictEqual(crate.trustpubOnly, false);

  await db.version.create({ crate, num: '1.0.0' });
  await db.crateOwnership.create({ crate, user });

  let response = await fetch('/api/v1/crates/foo', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ crate: { trustpub_only: true } }),
  });
  assert.strictEqual(response.status, 200);

  let json = await response.json();
  assert.strictEqual(json.crate.trustpub_only, true);

  let updatedCrate = db.crate.findFirst(q => q.where({ name: 'foo' }));
  assert.strictEqual(updatedCrate.trustpubOnly, true);
});
