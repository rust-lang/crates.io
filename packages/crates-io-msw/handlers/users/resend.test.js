import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns `ok`', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/users/${user.id}/resend`, { method: 'PUT' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });
});

test('returns 403 when not logged in', async function () {
  let user = db.user.create();

  let response = await fetch(`/api/v1/users/${user.id}/resend`, { method: 'PUT' });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'must be logged in to perform that action' }] });
});

test('returns 400 when requesting the wrong user id', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/users/wrong-id/resend`, { method: 'PUT' });
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'current user does not match requested user' }] });
});
