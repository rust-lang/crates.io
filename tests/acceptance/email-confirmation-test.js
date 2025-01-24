import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Email Confirmation', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  test('unauthenticated happy path', async function (assert) {
    let user = this.db.user.create({ emailVerificationToken: 'badc0ffee' });
    assert.false(user.emailVerified);

    await visit('/confirm/badc0ffee');
    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-notification-message="success"]').exists();

    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.true(user.emailVerified);
  });

  test('authenticated happy path', async function (assert) {
    let user = this.db.user.create({ emailVerificationToken: 'badc0ffee' });
    assert.false(user.emailVerified);

    this.authenticateAs(user);

    await visit('/confirm/badc0ffee');
    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-notification-message="success"]').exists();

    let { currentUser } = this.owner.lookup('service:session');
    assert.true(currentUser.email_verified);

    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.true(user.emailVerified);
  });

  test('error case', async function (assert) {
    await visit('/confirm/badc0ffee');
    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-notification-message]').hasText('Unknown error in email confirmation');
  });
});
