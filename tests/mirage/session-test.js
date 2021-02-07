import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from 'cargo/tests/helpers';

import setupMirage from '../helpers/setup-mirage';

module('Mirage | Session', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('DELETE /api/private/session', function () {
    test('returns 200 when authenticated', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let response = await fetch('/api/private/session', { method: 'DELETE' });
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { ok: true });

      assert.notOk(this.server.schema.mirageSessions.first());
    });

    test('returns 200 when unauthenticated', async function (assert) {
      let response = await fetch('/api/private/session', { method: 'DELETE' });
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { ok: true });

      assert.notOk(this.server.schema.mirageSessions.first());
    });
  });
});
