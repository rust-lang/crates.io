import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | DELETE /api/v1/me/tokens/:tokenId', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('revokes an API token', async function (assert) {
    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    let token = this.server.create('api-token', { user });

    let response = await fetch(`/api/v1/me/tokens/${token.id}`, { method: 'DELETE' });
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {});

    let tokens = this.server.schema.apiTokens.all().models;
    assert.equal(tokens.length, 0);
  });

  test('returns an error if unauthenticated', async function (assert) {
    let user = this.server.create('user');
    let token = this.server.create('api-token', { user });

    let response = await fetch(`/api/v1/me/tokens/${token.id}`, { method: 'DELETE' });
    assert.equal(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });
});
