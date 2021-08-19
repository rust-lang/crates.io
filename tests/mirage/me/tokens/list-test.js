import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | /me', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('GET /api/v1/me/tokens', function () {
    test('returns the list of API token for the authenticated `user`', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      this.server.create('api-token', { user, createdAt: '2017-11-19T12:59:22Z' });
      this.server.create('api-token', { user, createdAt: '2017-11-19T13:59:22Z' });
      this.server.create('api-token', { user, createdAt: '2017-11-19T14:59:22Z' });

      let response = await fetch('/api/v1/me/tokens');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        api_tokens: [
          {
            id: 3,
            created_at: '2017-11-19T14:59:22.000Z',
            last_used_at: null,
            name: 'API Token 3',
          },
          {
            id: 2,
            created_at: '2017-11-19T13:59:22.000Z',
            last_used_at: null,
            name: 'API Token 2',
          },
          {
            id: 1,
            created_at: '2017-11-19T12:59:22.000Z',
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
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { api_tokens: [] });
    });

    test('returns an error if unauthenticated', async function (assert) {
      let response = await fetch('/api/v1/me/tokens');
      assert.equal(response.status, 403);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        errors: [{ detail: 'must be logged in to perform that action' }],
      });
    });
  });
});
