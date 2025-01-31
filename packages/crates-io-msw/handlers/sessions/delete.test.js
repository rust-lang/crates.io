import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 200 when authenticated', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/private/session', { method: 'DELETE' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });

  assert.notOk(db.mswSession.findFirst({}));
});

test('returns 200 when unauthenticated', async function () {
  let response = await fetch('/api/private/session', { method: 'DELETE' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });

  assert.notOk(db.mswSession.findFirst({}));
});
