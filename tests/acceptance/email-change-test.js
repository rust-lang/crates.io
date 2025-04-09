import { click, currentURL, fillIn } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Email Change', function (hooks) {
  setupApplicationTest(hooks);

  test('happy path', async function (assert) {
    let user = this.db.user.create({ email: 'old@email.com' });

    this.authenticateAs(user);

    await visit('/settings/profile');
    assert.strictEqual(currentURL(), '/settings/profile');
    assert.dom('[data-test-email-input]').exists();
    assert.dom('[data-test-email-input] [data-test-no-email]').doesNotExist();
    assert.dom('[data-test-email-input] [data-test-email-address]').includesText('old@email.com');
    assert.dom('[data-test-email-input] [data-test-verified]').exists();
    assert.dom('[data-test-email-input] [data-test-not-verified]').doesNotExist();
    assert.dom('[data-test-email-input] [data-test-verification-sent]').doesNotExist();
    assert.dom('[data-test-email-input] [data-test-resend-button]').doesNotExist();

    await click('[data-test-email-input] [data-test-edit-button]');
    assert.dom('[data-test-email-input] [data-test-input]').hasValue('old@email.com');
    assert.dom('[data-test-email-input] [data-test-save-button]').isEnabled();
    assert.dom('[data-test-email-input] [data-test-cancel-button]').isEnabled();

    await fillIn('[data-test-email-input] [data-test-input]', '');
    assert.dom('[data-test-email-input] [data-test-input]').hasValue('');
    assert.dom('[data-test-email-input] [data-test-save-button]').isDisabled();

    await fillIn('[data-test-email-input] [data-test-input]', 'new@email.com');
    assert.dom('[data-test-email-input] [data-test-input]').hasValue('new@email.com');
    assert.dom('[data-test-email-input] [data-test-save-button]').isEnabled();

    await click('[data-test-email-input] [data-test-save-button]');
    assert.dom('[data-test-email-input] [data-test-email-address]').includesText('new@email.com');
    assert.dom('[data-test-email-input] [data-test-verified]').doesNotExist();
    assert.dom('[data-test-email-input] [data-test-not-verified]').exists();
    assert.dom('[data-test-email-input] [data-test-verification-sent]').exists();
    assert.dom('[data-test-email-input] [data-test-resend-button]').isEnabled();

    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.strictEqual(user.email, 'new@email.com');
    assert.false(user.emailVerified);
    assert.ok(user.emailVerificationToken);
  });

  test('happy path with `email: null`', async function (assert) {
    let user = this.db.user.create({ email: undefined });

    this.authenticateAs(user);

    await visit('/settings/profile');
    assert.strictEqual(currentURL(), '/settings/profile');
    assert.dom('[data-test-email-input]').exists();
    assert.dom('[data-test-email-input] [data-test-no-email]').exists();
    assert.dom('[data-test-email-input] [data-test-email-address]').hasText('');
    assert.dom('[data-test-email-input] [data-test-not-verified]').doesNotExist();
    assert.dom('[data-test-email-input] [data-test-verification-sent]').doesNotExist();
    assert.dom('[data-test-email-input] [data-test-resend-button]').doesNotExist();

    await click('[data-test-email-input] [data-test-edit-button]');
    assert.dom('[data-test-email-input] [data-test-input]').hasValue('');
    assert.dom('[data-test-email-input] [data-test-save-button]').isDisabled();
    assert.dom('[data-test-email-input] [data-test-cancel-button]').isEnabled();

    await fillIn('[data-test-email-input] [data-test-input]', 'new@email.com');
    assert.dom('[data-test-email-input] [data-test-input]').hasValue('new@email.com');
    assert.dom('[data-test-email-input] [data-test-save-button]').isEnabled();

    await click('[data-test-email-input] [data-test-save-button]');
    assert.dom('[data-test-email-input] [data-test-no-email]').doesNotExist();
    assert.dom('[data-test-email-input] [data-test-email-address]').includesText('new@email.com');
    assert.dom('[data-test-email-input] [data-test-verified]').doesNotExist();
    assert.dom('[data-test-email-input] [data-test-not-verified]').exists();
    assert.dom('[data-test-email-input] [data-test-verification-sent]').exists();
    assert.dom('[data-test-email-input] [data-test-resend-button]').isEnabled();

    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.strictEqual(user.email, 'new@email.com');
    assert.false(user.emailVerified);
    assert.ok(user.emailVerificationToken);
  });

  test('cancel button', async function (assert) {
    let user = this.db.user.create({ email: 'old@email.com' });

    this.authenticateAs(user);

    await visit('/settings/profile');
    await click('[data-test-email-input] [data-test-edit-button]');
    await fillIn('[data-test-email-input] [data-test-input]', 'new@email.com');
    assert.dom('[data-test-email-input] [data-test-invalid-email-warning]').doesNotExist();

    await click('[data-test-email-input] [data-test-cancel-button]');
    assert.dom('[data-test-email-input] [data-test-email-address]').includesText('old@email.com');
    assert.dom('[data-test-email-input] [data-test-verified]').exists();
    assert.dom('[data-test-email-input] [data-test-not-verified]').doesNotExist();
    assert.dom('[data-test-email-input] [data-test-verification-sent]').doesNotExist();

    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.strictEqual(user.email, 'old@email.com');
    assert.true(user.emailVerified);
    assert.notOk(user.emailVerificationToken);
  });

  test('server error', async function (assert) {
    let user = this.db.user.create({ email: 'old@email.com' });

    this.authenticateAs(user);

    this.worker.use(http.put('/api/v1/users/:user_id', () => HttpResponse.json({}, { status: 500 })));

    await visit('/settings/profile');
    await click('[data-test-email-input] [data-test-edit-button]');
    await fillIn('[data-test-email-input] [data-test-input]', 'new@email.com');

    await click('[data-test-email-input] [data-test-save-button]');
    assert.dom('[data-test-email-input] [data-test-input]').hasValue('new@email.com');
    assert.dom('[data-test-email-input] [data-test-email-address]').doesNotExist();
    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Error in saving email: An unknown error occurred while saving this email.');

    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.strictEqual(user.email, 'old@email.com');
    assert.true(user.emailVerified);
    assert.notOk(user.emailVerificationToken);
  });

  module('Resend button', function () {
    test('happy path', async function (assert) {
      let user = this.db.user.create({ email: 'john@doe.com', emailVerificationToken: 'secret123' });

      this.authenticateAs(user);

      await visit('/settings/profile');
      assert.strictEqual(currentURL(), '/settings/profile');
      assert.dom('[data-test-email-input]').exists();
      assert.dom('[data-test-email-input] [data-test-email-address]').includesText('john@doe.com');
      assert.dom('[data-test-email-input] [data-test-verified]').doesNotExist();
      assert.dom('[data-test-email-input] [data-test-not-verified]').exists();
      assert.dom('[data-test-email-input] [data-test-verification-sent]').exists();
      assert.dom('[data-test-email-input] [data-test-resend-button]').isEnabled().hasText('Resend');

      await click('[data-test-email-input] [data-test-resend-button]');
      assert.dom('[data-test-email-input] [data-test-resend-button]').isDisabled().hasText('Sent!');
    });

    test('server error', async function (assert) {
      let user = this.db.user.create({ email: 'john@doe.com', emailVerificationToken: 'secret123' });

      this.authenticateAs(user);

      this.worker.use(http.put('/api/v1/users/:user_id/resend', () => HttpResponse.json({}, { status: 500 })));

      await visit('/settings/profile');
      assert.strictEqual(currentURL(), '/settings/profile');
      assert.dom('[data-test-email-input]').exists();
      assert.dom('[data-test-email-input] [data-test-email-address]').includesText('john@doe.com');
      assert.dom('[data-test-email-input] [data-test-verified]').doesNotExist();
      assert.dom('[data-test-email-input] [data-test-not-verified]').exists();
      assert.dom('[data-test-email-input] [data-test-verification-sent]').exists();
      assert.dom('[data-test-email-input] [data-test-resend-button]').isEnabled().hasText('Resend');

      await click('[data-test-email-input] [data-test-resend-button]');
      assert.dom('[data-test-email-input] [data-test-resend-button]').isEnabled().hasText('Resend');
      assert.dom('[data-test-notification-message="error"]').hasText('Unknown error in resending message');
    });
  });
});
