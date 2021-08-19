import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | DELETE /api/private/session', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 200 when authenticated', async function (assert) {
    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    let response = await fetch('/api/private/session', { method: 'DELETE' });
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { ok: true });

    assert.notOk(this.server.schema.mirageSessions.first());
  });

  test('returns 200 when unauthenticated', async function (assert) {
    let response = await fetch('/api/private/session', { method: 'DELETE' });
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { ok: true });

    assert.notOk(this.server.schema.mirageSessions.first());
  });
});
