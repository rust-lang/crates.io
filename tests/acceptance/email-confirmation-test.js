import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Email Confirmation', function (hooks) {
  setupApplicationTest(hooks);

  test('unauthenticated happy path', async function (assert) {
    let user = this.server.create('user', { emailVerificationToken: 'badc0ffee' });
    assert.strictEqual(user.emailVerified, false);

    await visit('/confirm/badc0ffee');
    assert.equal(currentURL(), '/');
    assert.dom('[data-test-notification-message="success"]').exists();

    user.reload();
    assert.strictEqual(user.emailVerified, true);
  });

  test('authenticated happy path', async function (assert) {
    let user = this.server.create('user', { emailVerificationToken: 'badc0ffee' });
    assert.strictEqual(user.emailVerified, false);

    this.authenticateAs(user);

    await visit('/confirm/badc0ffee');
    assert.equal(currentURL(), '/');
    assert.dom('[data-test-notification-message="success"]').exists();

    let { currentUser } = this.owner.lookup('service:session');
    assert.strictEqual(currentUser.email_verified, true);

    user.reload();
    assert.strictEqual(user.emailVerified, true);
  });

  test('error case', async function (assert) {
    await visit('/confirm/badc0ffee');
    assert.equal(currentURL(), '/');
    assert.dom('[data-test-notification-message]').hasText('Unknown error in email confirmation');
  });
});
