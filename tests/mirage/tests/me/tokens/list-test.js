import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../../helpers';
import setupMirage from '../../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/me/tokens', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns the list of API token for the authenticated `user`', async function (assert) {
    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    this.server.create('api-token', {
      user,
      createdAt: '2017-11-19T12:59:22Z',
      crateScopes: ['serde', 'serde-*'],
      endpointScopes: ['publish-update'],
    });
    this.server.create('api-token', { user, createdAt: '2017-11-19T13:59:22Z', expiredAt: '2023-11-20T10:59:22Z' });
    this.server.create('api-token', { user, createdAt: '2017-11-19T14:59:22Z' });
    this.server.create('api-token', { user, createdAt: '2017-11-19T15:59:22Z', expiredAt: '2017-11-20T10:59:22Z' });

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

  test('empty list case', async function (assert) {
    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    let response = await fetch('/api/v1/me/tokens');
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), { api_tokens: [] });
  });

  test('returns an error if unauthenticated', async function (assert) {
    let response = await fetch('/api/v1/me/tokens');
    assert.strictEqual(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });
});
