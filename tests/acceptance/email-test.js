import { click, currentURL, fillIn } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Email Management', function (hooks) {
  setupApplicationTest(hooks);

  module('Add email', function () {
    test('happy path', async function (assert) {
      let user = this.db.user.create({ emails: [this.db.email.create({ email: 'old@email.com' })] });
      assert.strictEqual(user.emails[0].email, 'old@email.com');
      assert.false(user.emails[0].verified);

      this.authenticateAs(user);

      await visit('/settings/profile');
      assert.strictEqual(currentURL(), '/settings/profile');
      assert.dom('[data-test-add-email-button]').exists();
      assert.dom('[data-test-add-email-input]').doesNotExist();

      await click('[data-test-add-email-button]');
      assert.dom('[data-test-add-email-input]').exists();
      assert.dom('[data-test-add-email-input] [data-test-unverified]').doesNotExist();
      assert.dom('[data-test-add-email-input] [data-test-verified]').doesNotExist();
      assert.dom('[data-test-add-email-input] [data-test-verification-sent]').doesNotExist();
      assert.dom('[data-test-add-email-input] [data-test-resend-button]').doesNotExist();

      await fillIn('[data-test-add-email-input] [data-test-input]', '');
      assert.dom('[data-test-add-email-input] [data-test-input]').hasValue('');
      assert.dom('[data-test-add-email-input] [data-test-save-button]').isDisabled();

      await fillIn('[data-test-add-email-input] [data-test-input]', 'notanemail');
      assert.dom('[data-test-add-email-input] [data-test-input]').hasValue('notanemail');
      assert.dom('[data-test-add-email-input] [data-test-save-button]').isDisabled();

      await fillIn('[data-test-add-email-input] [data-test-input]', 'new@email.com');
      assert.dom('[data-test-add-email-input] [data-test-input]').hasValue('new@email.com');
      assert.dom('[data-test-add-email-input] [data-test-save-button]').isEnabled();

      await click('[data-test-add-email-input] [data-test-save-button]');
      assert.dom('[data-test-add-email-button]').exists();
      assert.dom('[data-test-add-email-input]').doesNotExist();

      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-email-address]').includesText('old@email.com');
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-email-address]').includesText('new@email.com');
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-verified]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-unverified]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-verification-sent]').exists();

      user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
      assert.strictEqual(user.emails[0].email, 'old@email.com');
      assert.strictEqual(user.emails[1].email, 'new@email.com');
      assert.false(user.emails[1].verified);
    });

    test('happy path with no previous emails', async function (assert) {
      let user = this.db.user.create({ emails: [] });
      assert.strictEqual(user.emails.length, 0);

      this.authenticateAs(user);

      await visit('/settings/profile');
      assert.strictEqual(currentURL(), '/settings/profile');
      assert.dom('[data-test-add-email-button]').exists();
      assert.dom('[data-test-add-email-input]').doesNotExist();

      await click('[data-test-add-email-button]');
      assert.dom('[data-test-add-email-input]').exists();

      await fillIn('[data-test-add-email-input] [data-test-input]', 'new@email.com');
      await click('[data-test-add-email-input] [data-test-save-button]');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-email-address]').includesText('new@email.com');

      user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
      assert.strictEqual(user.emails.length, 1);
      assert.strictEqual(user.emails[0].email, 'new@email.com');
    });

    test('server error', async function (assert) {
      let user = this.db.user.create({ emails: [this.db.email.create({ email: 'old@email.com' })] });

      this.authenticateAs(user);

      this.worker.use(http.post('/api/v1/users/:user_id/emails', () => HttpResponse.json({}, { status: 500 })));

      await visit('/settings/profile');
      await click('[data-test-add-email-button]');
      assert.dom('[data-test-add-email-input]').exists();

      await fillIn('[data-test-add-email-input] [data-test-input]', 'new@email.com');
      await click('[data-test-add-email-input] [data-test-save-button]');
      assert.dom('[data-test-notification-message="error"]').hasText('Unknown error in saving email');

      user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
      assert.strictEqual(user.emails[0].email, 'old@email.com');
      assert.strictEqual(user.emails.length, 1);
    });
  });

  module('Remove email', function () {
    test('happy path', async function (assert) {
      let user = this.db.user.create({
        emails: [this.db.email.create({ email: 'john@doe.com' }), this.db.email.create({ email: 'jane@doe.com' })],
      });

      this.authenticateAs(user);

      await visit('/settings/profile');
      assert.strictEqual(currentURL(), '/settings/profile');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-email-address]').includesText('john@doe.com');
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-email-address]').includesText('jane@doe.com');

      await click('[data-test-email-input]:nth-of-type(2) [data-test-remove-button]');
      assert.dom('[data-test-email-input]').exists({ count: 1 });
      assert.dom('[data-test-email-input] [data-test-remove-button]').doesNotExist();

      user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
      assert.strictEqual(user.emails[0].email, 'john@doe.com');
      assert.strictEqual(user.emails.length, 1);
    });

    test('cannot remove primary email', async function (assert) {
      let user = this.db.user.create({
        emails: [
          this.db.email.create({ email: 'primary@doe.com', primary: true }),
          this.db.email.create({ email: 'john@doe.com' }),
        ],
      });
      this.authenticateAs(user);
      await visit('/settings/profile');
      assert.strictEqual(currentURL(), '/settings/profile');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-email-address]').includesText('primary@doe.com');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-remove-button]').isDisabled();
      assert
        .dom('[data-test-email-input]:nth-of-type(1) [data-test-remove-button]')
        .hasAttribute('title', 'Cannot delete primary email');
    });

    test('no delete button when only one email', async function (assert) {
      let user = this.db.user.create({ emails: [this.db.email.create({ email: 'john@doe.com' })] });
      this.authenticateAs(user);
      await visit('/settings/profile');
      assert.strictEqual(currentURL(), '/settings/profile');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-email-address]').includesText('john@doe.com');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-remove-button]').doesNotExist();
    });

    test('server error', async function (assert) {
      let user = this.db.user.create({
        emails: [this.db.email.create({ email: 'john@doe.com' }), this.db.email.create({ email: 'jane@doe.com' })],
      });

      this.authenticateAs(user);

      this.worker.use(
        http.delete('/api/v1/users/:user_id/emails/:email_id', () => HttpResponse.json({}, { status: 500 })),
      );

      await visit('/settings/profile');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-email-address]').includesText('john@doe.com');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-remove-button]').exists();
      await click('[data-test-email-input]:nth-of-type(1) [data-test-remove-button]');
      assert.dom('[data-test-notification-message="error"]').hasText('Unknown error in deleting email');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-remove-button]').isEnabled();
    });
  });

  module('Resend verification email', function () {
    test('happy path', async function (assert) {
      let user = this.db.user.create({
        emails: [this.db.email.create({ email: 'john@doe.com', verified: false, verification_email_sent: true })],
      });

      this.authenticateAs(user);

      await visit('/settings/profile');
      assert.strictEqual(currentURL(), '/settings/profile');
      assert.dom('[data-test-email-input]').exists();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-email-address]').includesText('john@doe.com');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-verified]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-unverified]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-verification-sent]').exists();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-resend-button]').isEnabled().hasText('Resend');

      await click('[data-test-email-input] [data-test-resend-button]');
      assert.dom('[data-test-email-input] [data-test-resend-button]').isDisabled().hasText('Sent!');
    });

    test('server error', async function (assert) {
      let user = this.db.user.create({
        emails: [this.db.email.create({ email: 'john@doe.com', verified: false, verification_email_sent: true })],
      });

      this.authenticateAs(user);

      this.worker.use(
        http.put('/api/v1/users/:user_id/emails/:email_id/resend', () => HttpResponse.json({}, { status: 500 })),
      );

      await visit('/settings/profile');
      assert.strictEqual(currentURL(), '/settings/profile');
      assert.dom('[data-test-email-input]').exists();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-email-address]').includesText('john@doe.com');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-verified]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-unverified]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-verification-sent]').exists();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-resend-button]').isEnabled().hasText('Resend');

      await click('[data-test-email-input]:nth-of-type(1) [data-test-resend-button]');
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-resend-button]').isEnabled().hasText('Resend');
      assert.dom('[data-test-notification-message="error"]').hasText('Unknown error in resending message');
    });
  });

  module('Switch primary email', function () {
    test('happy path', async function (assert) {
      let user = this.db.user.create({
        emails: [
          this.db.email.create({ email: 'john@doe.com', verified: true, primary: true }),
          this.db.email.create({ email: 'jane@doe.com', verified: true, primary: false }),
        ],
      });
      this.authenticateAs(user);

      await visit('/settings/profile');

      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-email-address]').includesText('john@doe.com');
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-email-address]').includesText('jane@doe.com');

      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-primary]').isVisible();
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-primary]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-primary-button]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-primary-button]').isEnabled();

      await click('[data-test-email-input]:nth-of-type(2) [data-test-primary-button]');

      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-primary]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-primary]').isVisible();
      assert.dom('[data-test-email-input]:nth-of-type(2) [data-test-primary-button]').doesNotExist();
      assert.dom('[data-test-email-input]:nth-of-type(1) [data-test-primary-button]').isEnabled();
    });
  });
});
