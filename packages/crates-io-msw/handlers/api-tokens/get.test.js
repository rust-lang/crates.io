import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns the requested token', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let token = db.apiToken.create({
    user,
    crateScopes: ['serde', 'serde-*'],
    endpointScopes: ['publish-update'],
  });

  let response = await fetch(`/api/v1/me/tokens/${token.id}`);
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    api_token: {
      id: 1,
      crate_scopes: ['serde', 'serde-*'],
      created_at: '2017-11-19T17:59:22.000Z',
      endpoint_scopes: ['publish-update'],
      expired_at: null,
      last_used_at: null,
      name: 'API Token 1',
    },
  });
});

test('returns 404 if token not found', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/me/tokens/42');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns an error if unauthenticated', async function () {
  let response = await fetch('/api/v1/me/tokens/42');
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
