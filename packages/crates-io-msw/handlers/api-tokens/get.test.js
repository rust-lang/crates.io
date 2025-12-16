import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns the requested token', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let token = await db.apiToken.create({
    user,
    crateScopes: ['serde', 'serde-*'],
    endpointScopes: ['publish-update'],
  });

  let response = await fetch(`/api/v1/me/tokens/${token.id}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
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
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/me/tokens/42');
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({ errors: [{ detail: 'Not Found' }] });
});

test('returns an error if unauthenticated', async function () {
  let response = await fetch('/api/v1/me/tokens/42');
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
