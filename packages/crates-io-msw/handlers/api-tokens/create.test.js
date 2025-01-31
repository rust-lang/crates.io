import { afterEach, assert, beforeEach, test, vi } from 'vitest';

import { db } from '../../index.js';

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2017-11-20T11:23:45Z'));
});

afterEach(() => {
  vi.restoreAllMocks();
});

test('creates a new API token', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let body = JSON.stringify({ api_token: { name: 'foooo' } });
  let response = await fetch('/api/v1/me/tokens', { method: 'PUT', body });
  assert.strictEqual(response.status, 200);

  let token = db.apiToken.findMany({})[0];
  assert.ok(token);

  assert.deepEqual(await response.json(), {
    api_token: {
      id: 1,
      crate_scopes: null,
      created_at: '2017-11-20T11:23:45.000Z',
      endpoint_scopes: null,
      expired_at: null,
      last_used_at: null,
      name: 'foooo',
      revoked: false,
      token: token.token,
    },
  });
});

test('creates a new API token with scopes', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let body = JSON.stringify({
    api_token: {
      name: 'foooo',
      crate_scopes: ['serde', 'serde-*'],
      endpoint_scopes: ['publish-update'],
    },
  });
  let response = await fetch('/api/v1/me/tokens', { method: 'PUT', body });
  assert.strictEqual(response.status, 200);

  let token = db.apiToken.findMany({})[0];
  assert.ok(token);

  assert.deepEqual(await response.json(), {
    api_token: {
      id: 1,
      crate_scopes: ['serde', 'serde-*'],
      created_at: '2017-11-20T11:23:45.000Z',
      endpoint_scopes: ['publish-update'],
      expired_at: null,
      last_used_at: null,
      name: 'foooo',
      revoked: false,
      token: token.token,
    },
  });
});

test('creates a new API token with expiry date', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let body = JSON.stringify({
    api_token: {
      name: 'foooo',
      expired_at: '2023-12-24T12:34:56Z',
    },
  });
  let response = await fetch('/api/v1/me/tokens', { method: 'PUT', body });
  assert.strictEqual(response.status, 200);

  let token = db.apiToken.findMany({})[0];
  assert.ok(token);

  assert.deepEqual(await response.json(), {
    api_token: {
      id: 1,
      crate_scopes: null,
      created_at: '2017-11-20T11:23:45.000Z',
      endpoint_scopes: null,
      expired_at: '2023-12-24T12:34:56.000Z',
      last_used_at: null,
      name: 'foooo',
      revoked: false,
      token: token.token,
    },
  });
});

test('returns an error if unauthenticated', async function () {
  let body = JSON.stringify({ api_token: {} });
  let response = await fetch('/api/v1/me/tokens', { method: 'PUT', body });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
