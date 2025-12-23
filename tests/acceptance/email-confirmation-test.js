import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Email Confirmation', function (hooks) {
  setupApplicationTest(hooks);

  test('unauthenticated happy path', async function (assert) {
    let user = await this.db.user.create({ emailVerificationToken: 'badc0ffee' });
    assert.false(user.emailVerified);

    await visit('/confirm/badc0ffee');
    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-notification-message="success"]').exists();

    user = this.db.user.findFirst(q => q.where({ id: user.id }));
    assert.true(user.emailVerified);
  });

  test('authenticated happy path', async function (assert) {
    let user = await this.db.user.create({ emailVerificationToken: 'badc0ffee' });
    assert.false(user.emailVerified);

    await this.authenticateAs(user);

    await visit('/confirm/badc0ffee');
    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-notification-message="success"]').exists();

    user = this.db.user.findFirst(q => q.where({ id: user.id }));
    assert.true(user.emailVerified);

    await visit('/settings/profile');
    assert.dom('[data-test-verified]').exists();
  });

  test('error case', async function (assert) {
    await visit('/confirm/badc0ffee');
    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-notification-message]').hasText('Unknown error in email confirmation');
  });
});
