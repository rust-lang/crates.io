import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { currentURL } from '@ember/test-helpers';
import window, { setupWindowMock } from 'ember-window-mock';
import { Response } from 'ember-cli-mirage';

import { visit } from '../helpers/visit-ignoring-abort';
import setupMirage from '../helpers/setup-mirage';

module('Acceptance | Email Confirmation', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);
  setupMirage(hooks);

  test('unauthenticated happy path', async function (assert) {
    assert.expect(3);

    this.server.put('/api/v1/confirm/:token', (schema, request) => {
      assert.equal(request.params.token, 'badc0ffee');
      return { ok: true };
    });

    await visit('/confirm/badc0ffee');
    assert.equal(currentURL(), '/confirm/badc0ffee');
    assert.dom('[data-test-success-message]').exists();
  });

  test('authenticated happy path', async function (assert) {
    assert.expect(4);

    let emailVerified = false;

    window.localStorage.setItem('isLoggedIn', '1');

    this.server.get('/api/v1/me', () => ({
      user: {
        id: 42,
        login: 'johnnydee',
        email_verified: emailVerified,
        email_verification_sent: true,
        name: 'John Doe',
        email: 'john@doe.com',
        avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
        url: 'https://github.com/johnnydee',
      },
      owned_crates: [],
    }));

    this.server.put('/api/v1/confirm/:token', (schema, request) => {
      assert.equal(request.params.token, 'badc0ffee');

      emailVerified = true;

      return { ok: true };
    });

    await visit('/confirm/badc0ffee');
    assert.equal(currentURL(), '/confirm/badc0ffee');
    assert.dom('[data-test-success-message]').exists();

    let { currentUser } = this.owner.lookup('service:session');
    assert.strictEqual(currentUser.email_verified, true);
  });

  test('error case', async function (assert) {
    this.server.put('/api/v1/confirm/:token', () => {
      return new Response(400, {}, { errors: [{ detail: 'Email belonging to token not found.' }] });
    });

    await visit('/confirm/badc0ffee');
    assert.equal(currentURL(), '/');
    assert.dom('[data-test-flash-message]').hasText('Unknown error in email confirmation');
  });
});
