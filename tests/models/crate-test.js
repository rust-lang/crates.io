import { module, test } from 'qunit';

import AdapterError from '@ember-data/adapter/error';

import { setupMirage } from 'ember-cli-mirage/test-support';

import { setupTest } from 'cargo/tests/helpers';

module('Model | Crate', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  hooks.beforeEach(function () {
    this.store = this.owner.lookup('service:store');
  });

  module('inviteOwner()', function () {
    test('happy path', async function (assert) {
      let user = this.server.create('user');

      let crate = this.server.create('crate');
      this.server.create('version', { crate });

      let crateRecord = await this.store.findRecord('crate', crate.name);

      let result = await crateRecord.inviteOwner(user.login);
      assert.deepEqual(result, { ok: true });
    });

    test('error handling', async function (assert) {
      let crate = this.server.create('crate');
      this.server.create('version', { crate });

      let crateRecord = await this.store.findRecord('crate', crate.name);

      await assert.rejects(crateRecord.inviteOwner('unknown'), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'could not find user with login `unknown`' }]);
        return true;
      });
    });
  });

  module('removeOwner()', function () {
    test('happy path', async function (assert) {
      let user = this.server.create('user');

      let crate = this.server.create('crate');
      this.server.create('version', { crate });

      let crateRecord = await this.store.findRecord('crate', crate.name);

      let result = await crateRecord.removeOwner(user.login);
      assert.deepEqual(result, { ok: true, msg: 'owners successfully removed' });
    });

    test('error handling', async function (assert) {
      let crate = this.server.create('crate');
      this.server.create('version', { crate });

      let crateRecord = await this.store.findRecord('crate', crate.name);

      await assert.rejects(crateRecord.removeOwner('unknown'), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'Not Found' }]);
        return error instanceof AdapterError;
      });
    });
  });
});
