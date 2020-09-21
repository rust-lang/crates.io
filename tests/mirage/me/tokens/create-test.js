import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | PUT /api/v1/me/tokens', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('creates a new API token', async function (assert) {
    this.clock.setSystemTime(new Date('2017-11-20T11:23:45Z'));

    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    let body = JSON.stringify({ api_token: { name: 'foooo' } });
    let response = await fetch('/api/v1/me/tokens', { method: 'PUT', body });
    assert.equal(response.status, 200);

    let token = this.server.schema.apiTokens.all().models[0];
    assert.ok(token);

    assert.deepEqual(await response.json(), {
      api_token: {
        id: 1,
        created_at: '2017-11-20T11:23:45.000Z',
        last_used_at: null,
        name: 'foooo',
        revoked: false,
        token: token.token,
      },
    });
  });

  test('returns an error if unauthenticated', async function (assert) {
    let body = JSON.stringify({ api_token: {} });
    let response = await fetch('/api/v1/me/tokens', { method: 'PUT', body });
    assert.equal(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });
});
