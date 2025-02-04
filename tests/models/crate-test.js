import { module, test } from 'qunit';

import AdapterError from '@ember-data/adapter/error';

import { setupTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Model | Crate', function (hooks) {
  setupTest(hooks);
  setupMsw(hooks);

  hooks.beforeEach(function () {
    this.store = this.owner.lookup('service:store');
  });

  module('inviteOwner()', function () {
    test('happy path', async function (assert) {
      let user = this.db.user.create();
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });

      let user2 = this.db.user.create();

      let crateRecord = await this.store.findRecord('crate', crate.name);

      let result = await crateRecord.inviteOwner(user2.login);
      assert.deepEqual(result, { ok: true, msg: 'user user-2 has been invited to be an owner of crate crate-1' });
    });

    test('error handling', async function (assert) {
      let user = this.db.user.create();
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });

      let crateRecord = await this.store.findRecord('crate', crate.name);

      await assert.rejects(crateRecord.inviteOwner('unknown'), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'could not find user with login `unknown`' }]);
        return true;
      });
    });
  });

  module('removeOwner()', function () {
    test('happy path', async function (assert) {
      let user = this.db.user.create();
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });

      let user2 = this.db.user.create();
      this.db.crateOwnership.create({ crate, user: user2 });

      let crateRecord = await this.store.findRecord('crate', crate.name);

      let result = await crateRecord.removeOwner(user2.login);
      assert.deepEqual(result, { ok: true, msg: 'owners successfully removed' });
    });

    test('error handling', async function (assert) {
      let user = this.db.user.create();
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });

      let crateRecord = await this.store.findRecord('crate', crate.name);

      await assert.rejects(crateRecord.removeOwner('unknown'), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'Not Found' }]);
        return error instanceof AdapterError;
      });
    });
  });
});
