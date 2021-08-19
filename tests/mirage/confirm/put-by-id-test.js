import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | PUT /api/v1/confirm/:token', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns `ok: true` for a known token (unauthenticated)', async function (assert) {
    let user = this.server.create('user', { emailVerificationToken: 'foo' });
    assert.strictEqual(user.emailVerified, false);

    let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { ok: true });

    user.reload();
    assert.strictEqual(user.emailVerified, true);
  });

  test('returns `ok: true` for a known token (authenticated)', async function (assert) {
    let user = this.server.create('user', { emailVerificationToken: 'foo' });
    assert.strictEqual(user.emailVerified, false);

    this.server.create('mirage-session', { user });

    let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { ok: true });

    user.reload();
    assert.strictEqual(user.emailVerified, true);
  });

  test('returns an error for unknown tokens', async function (assert) {
    let response = await fetch('/api/v1/confirm/unknown', { method: 'PUT' });
    assert.equal(response.status, 400);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'Email belonging to token not found.' }],
    });
  });
});
