import { afterEach, assert, beforeEach, test, vi } from 'vitest';

import { db } from '../../index.js';

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2017-11-20T12:00:00Z'));
});

afterEach(() => {
  vi.restoreAllMocks();
});

test('returns the list of API token for the authenticated `user`', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  db.apiToken.create({
    user,
    createdAt: '2017-11-19T12:59:22Z',
    crateScopes: ['serde', 'serde-*'],
    endpointScopes: ['publish-update'],
  });
  db.apiToken.create({ user, createdAt: '2017-11-19T13:59:22Z', expiredAt: '2023-11-20T10:59:22Z' });
  db.apiToken.create({ user, createdAt: '2017-11-19T14:59:22Z' });
  db.apiToken.create({ user, createdAt: '2017-11-19T15:59:22Z', expiredAt: '2017-11-20T10:59:22Z' });

  let response = await fetch('/api/v1/me/tokens');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    api_tokens: [
      {
        id: 3,
        crate_scopes: null,
        created_at: '2017-11-19T14:59:22.000Z',
        endpoint_scopes: null,
        expired_at: null,
        last_used_at: null,
        name: 'API Token 3',
      },
      {
        id: 2,
        crate_scopes: null,
        created_at: '2017-11-19T13:59:22.000Z',
        endpoint_scopes: null,
        expired_at: '2023-11-20T10:59:22.000Z',
        last_used_at: null,
        name: 'API Token 2',
      },
      {
        id: 1,
        crate_scopes: ['serde', 'serde-*'],
        created_at: '2017-11-19T12:59:22.000Z',
        endpoint_scopes: ['publish-update'],
        expired_at: null,
        last_used_at: null,
        name: 'API Token 1',
      },
    ],
  });
});

test('empty list case', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/me/tokens');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { api_tokens: [] });
});

test('returns an error if unauthenticated', async function () {
  let response = await fetch('/api/v1/me/tokens');
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
