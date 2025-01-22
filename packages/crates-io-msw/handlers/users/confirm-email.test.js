import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns `ok: true` for a known token (unauthenticated)', async function () {
  let user = db.user.create({ emailVerificationToken: 'foo' });
  assert.strictEqual(user.emailVerified, false);

  let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });

  user = db.user.findFirst({ where: { id: user.id } });
  assert.strictEqual(user.emailVerified, true);
});

test('returns `ok: true` for a known token (authenticated)', async function () {
  let user = db.user.create({ emailVerificationToken: 'foo' });
  assert.strictEqual(user.emailVerified, false);

  db.mswSession.create({ user });

  let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });

  user = db.user.findFirst({ where: { id: user.id } });
  assert.strictEqual(user.emailVerified, true);
});

test('returns an error for unknown tokens', async function () {
  let response = await fetch('/api/v1/confirm/unknown', { method: 'PUT' });
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Email belonging to token not found.' }],
  });
});
