import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Email Confirmation', function (hooks) {
  setupApplicationTest(hooks);

  test('unauthenticated happy path', async function (assert) {
    let email = this.db.email.create({ verified: false, token: 'badc0ffee' });
    let user = this.db.user.create({ emails: [email] });
    assert.false(email.verified);

    await visit('/confirm/badc0ffee');
    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-notification-message="success"]').exists();

    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.true(user.emails[0].verified);
  });

  test('authenticated happy path', async function (assert) {
    let user = this.db.user.create({ emails: [this.db.email.create({ verified: false, token: 'badc0ffee' })] });
    assert.false(user.emails[0].verified);

    this.authenticateAs(user);

    await visit('/confirm/badc0ffee');
    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-notification-message="success"]').exists();

    let { currentUser } = this.owner.lookup('service:session');
    assert.true(currentUser.emails[0].verified);

    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.true(user.emails[0].verified);
  });

  test('error case', async function (assert) {
    await visit('/confirm/badc0ffee');
    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-notification-message]').hasText('Unknown error in email confirmation');
  });
});
