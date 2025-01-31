import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('revokes an API token', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let token = db.apiToken.create({ user });

  let response = await fetch(`/api/v1/me/tokens/${token.id}`, { method: 'DELETE' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {});

  let tokens = db.apiToken.findMany({});
  assert.strictEqual(tokens.length, 0);
});

test('returns an error if unauthenticated', async function () {
  let user = db.user.create();
  let token = db.apiToken.create({ user });

  let response = await fetch(`/api/v1/me/tokens/${token.id}`, { method: 'DELETE' });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
