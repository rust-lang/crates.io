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

  module('changeEmail()', function () {
    test('happy path', async function (assert) {
      let user = this.db.user.create({ email: 'old@email.com' });

      this.authenticateAs(user);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();
      assert.strictEqual(currentUser.email, 'old@email.com');
      assert.true(currentUser.email_verified);
      assert.true(currentUser.email_verification_sent);

      await currentUser.changeEmail('new@email.com');
      assert.strictEqual(currentUser.email, 'new@email.com');
      assert.false(currentUser.email_verified);
      assert.true(currentUser.email_verification_sent);
    });

    test('error handling', async function (assert) {
      let user = this.db.user.create({ email: 'old@email.com' });

      this.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      this.worker.use(http.put('/api/v1/users/:user_id', () => error));

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await assert.rejects(currentUser.changeEmail('new@email.com'), function (error) {
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

      let user = this.db.user.create({ emailVerificationToken: 'secret123' });
      this.authenticateAs(user);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await currentUser.resendVerificationEmail();
    });

    test('error handling', async function (assert) {
      let user = this.db.user.create({ emailVerificationToken: 'secret123' });
      this.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      this.worker.use(http.put('/api/v1/users/:user_id/resend', () => error));

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await assert.rejects(currentUser.resendVerificationEmail(), function (error) {
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
