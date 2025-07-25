import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Model | User', function (hooks) {
  setupTest(hooks);
  setupMsw(hooks);

  hooks.beforeEach(function () {
    this.store = this.owner.lookup('service:store');
  });

  module('addEmail()', function () {
    test('happy path', async function (assert) {
      let email = this.db.email.create({ email: 'old@email.com' });
      let user = this.db.user.create({ emails: [email] });

      this.authenticateAs(user);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();
      assert.strictEqual(currentUser.emails[0].email, 'old@email.com');

      await currentUser.addEmail('new@email.com');
      assert.strictEqual(currentUser.emails[1].email, 'new@email.com');
    });

    test('error handling', async function (assert) {
      let email = this.db.email.create({ email: 'old@email.com' });
      let user = this.db.user.create({ emails: [email] });

      this.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      this.worker.use(http.post('/api/v1/users/:user_id/emails', () => error));

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await assert.rejects(currentUser.addEmail('new@email.com'), function (error) {
        assert.deepEqual(error.errors, [
          {
            detail: '{}',
            status: '500',
            title: 'The backend responded with an error',
          },
        ]);
        return true;
      });
    });
  });

  module('deleteEmail()', function () {
    test('happy path', async function (assert) {
      let email = this.db.email.create({ email: 'old@email.com' });
      let user = this.db.user.create({ emails: [email] });
      this.authenticateAs(user);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await currentUser.deleteEmail(email.id);
      assert.false(currentUser.emails.some(e => e.id === email.id));
    });

    test('error handling', async function (assert) {
      let email = this.db.email.create({ email: 'old@email.com' });
      let user = this.db.user.create({ emails: [email] });
      this.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      this.worker.use(http.delete('/api/v1/users/:user_id/emails/:email_id', () => error));

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await assert.rejects(currentUser.deleteEmail(email.id), function (error) {
        assert.deepEqual(error.errors, [
          {
            detail: '{}',
            status: '500',
            title: 'The backend responded with an error',
          },
        ]);
        return true;
      });
    });
  });

  module('updateNotificationEmail()', function () {
    test('happy path', async function (assert) {
      let email = this.db.email.create({ email: 'old@email.com' });
      let user = this.db.user.create({ emails: [email] });
      this.authenticateAs(user);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await currentUser.updateNotificationEmail(email.id, 'new@email.com');
      assert.strictEqual(currentUser.emails.find(e => e.send_notifications).id, email.id);
    });
    test('error handling', async function (assert) {
      let email = this.db.email.create({ email: 'old@email.com' });
      let user = this.db.user.create({ emails: [email] });
      this.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      this.worker.use(http.put('/api/v1/users/:user_id/emails/:email_id/notifications', () => error));

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();
      await assert.rejects(currentUser.updateNotificationEmail(email.id, 'new@email.com'), function (error) {
        assert.deepEqual(error.errors, [
          {
            detail: '{}',
            status: '500',
            title: 'The backend responded with an error',
          },
        ]);
        return true;
      });
    });
  });

  module('resendVerificationEmail()', function () {
    test('happy path', async function (assert) {
      assert.expect(0);

      let email = this.db.email.create({ token: 'secret123' });
      let user = this.db.user.create({ emails: [email] });
      this.authenticateAs(user);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await currentUser.resendVerificationEmail(email.id);
    });

    test('error handling', async function (assert) {
      let email = this.db.email.create({ token: 'secret123' });
      let user = this.db.user.create({ emails: [email] });
      this.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      this.worker.use(http.put('/api/v1/users/:user_id/emails/:email_id/resend', () => error));

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await assert.rejects(currentUser.resendVerificationEmail(email.id), function (error) {
        assert.deepEqual(error.errors, [
          {
            detail: '{}',
            status: '500',
            title: 'The backend responded with an error',
          },
        ]);
        return true;
      });
    });
  });
});
