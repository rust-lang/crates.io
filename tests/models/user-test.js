import { module, test } from 'qunit';

import { setupTest } from 'cargo/tests/helpers';

import setupMirage from '../helpers/setup-mirage';

module('Model | User', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  hooks.beforeEach(function () {
    this.store = this.owner.lookup('service:store');
  });

  module('changeEmail()', function () {
    test('happy path', async function (assert) {
      let user = this.server.create('user', { email: 'old@email.com' });

      this.authenticateAs(user);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();
      assert.equal(currentUser.email, 'old@email.com');
      assert.equal(currentUser.email_verified, true);
      assert.equal(currentUser.email_verification_sent, true);

      await currentUser.changeEmail('new@email.com');
      assert.equal(currentUser.email, 'new@email.com');
      assert.equal(currentUser.email_verified, false);
      assert.equal(currentUser.email_verification_sent, true);
    });

    test('error handling', async function (assert) {
      let user = this.server.create('user', { email: 'old@email.com' });

      this.authenticateAs(user);

      this.server.put('/api/v1/users/:user_id', {}, 500);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await assert.rejects(currentUser.changeEmail('new@email.com'), function (error) {
        assert.deepEqual(error.errors, [
          {
            detail: '[object Object]',
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

      let user = this.server.create('user', { emailVerificationToken: 'secret123' });
      this.authenticateAs(user);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await currentUser.resendVerificationEmail();
    });

    test('error handling', async function (assert) {
      let user = this.server.create('user', { emailVerificationToken: 'secret123' });
      this.authenticateAs(user);

      this.server.put('/api/v1/users/:user_id/resend', {}, 500);

      let { currentUser } = await this.owner.lookup('service:session').loadUserTask.perform();

      await assert.rejects(currentUser.resendVerificationEmail(), function (error) {
        assert.deepEqual(error.errors, [
          {
            detail: '[object Object]',
            status: '500',
            title: 'The backend responded with an error',
          },
        ]);
        return true;
      });
    });
  });
});
