import { module, test } from 'qunit';

import { db } from '@crates-io/msw';

import { setupTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Model | TrustpubGitHubConfig', function (hooks) {
  setupTest(hooks);
  setupMsw(hooks);

  hooks.beforeEach(function () {
    this.store = this.owner.lookup('service:store');
  });

  module('query()', function () {
    test('fetches GitHub configs for a crate', async function (assert) {
      let user = this.db.user.create();
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });
      this.db.crateOwnership.create({ crate, user });

      let config = this.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      let configs = await this.store.query('trustpub-github-config', { crate: crate.name });
      assert.strictEqual(configs.length, 1);
      assert.strictEqual(parseInt(configs[0].id, 10), config.id);
      assert.strictEqual(configs[0].repository_owner, 'rust-lang');
      assert.strictEqual(configs[0].repository_name, 'crates.io');
      assert.strictEqual(configs[0].workflow_filename, 'ci.yml');
      assert.true(configs[0].created_at instanceof Date);
    });

    test('returns an error if the user is not authenticated', async function (assert) {
      let crate = this.db.crate.create();
      this.db.version.create({ crate });

      await assert.rejects(this.store.query('trustpub-github-config', { crate: crate.name }), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'must be logged in to perform that action' }]);
        return true;
      });
    });

    test('returns an error if the user is not an owner of the crate', async function (assert) {
      let user = this.db.user.create();
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });

      await assert.rejects(this.store.query('trustpub-github-config', { crate: crate.name }), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'You are not an owner of this crate' }]);
        return true;
      });
    });
  });

  module('createRecord()', function () {
    test('creates a new GitHub config', async function (assert) {
      let user = this.db.user.create({ emails: [this.db.email.create({ verified: true })] });
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });
      this.db.crateOwnership.create({ crate, user });

      let config = this.store.createRecord('trustpub-github-config', {
        crate: await this.store.findRecord('crate', crate.name),
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      await config.save();
      assert.strictEqual(config.id, '1');
      assert.strictEqual(config.repository_owner, 'rust-lang');
      assert.strictEqual(config.repository_name, 'crates.io');
      assert.strictEqual(config.workflow_filename, 'ci.yml');
    });

    test('returns an error if the user is not authenticated', async function (assert) {
      let crate = this.db.crate.create();
      this.db.version.create({ crate });

      let config = this.store.createRecord('trustpub-github-config', {
        crate: await this.store.findRecord('crate', crate.name),
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      await assert.rejects(config.save(), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'must be logged in to perform that action' }]);
        return true;
      });
    });

    test('returns an error if the user is not an owner of the crate', async function (assert) {
      let user = this.db.user.create({ emails: [this.db.email.create({ verified: true })] });
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });

      let config = this.store.createRecord('trustpub-github-config', {
        crate: await this.store.findRecord('crate', crate.name),
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      await assert.rejects(config.save(), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'You are not an owner of this crate' }]);
        return true;
      });
    });

    test('returns an error if the user does not have a verified email', async function (assert) {
      let user = this.db.user.create({ emails: [this.db.email.create({ verified: false })] });
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });
      this.db.crateOwnership.create({ crate, user });

      let config = this.store.createRecord('trustpub-github-config', {
        crate: await this.store.findRecord('crate', crate.name),
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      await assert.rejects(config.save(), function (error) {
        let detail = 'You must verify your email address to create a Trusted Publishing config';
        assert.deepEqual(error.errors, [{ detail }]);
        return true;
      });
    });
  });

  module('deleteRecord()', function () {
    test('deletes a GitHub config', async function (assert) {
      let user = this.db.user.create();
      this.authenticateAs(user);

      let crate = this.db.crate.create();
      this.db.version.create({ crate });
      this.db.crateOwnership.create({ crate, user });

      // Create a config in the database that will be queried later
      this.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      let configs = await this.store.query('trustpub-github-config', { crate: crate.name });
      assert.strictEqual(configs.length, 1);

      await configs[0].destroyRecord();

      configs = await this.store.query('trustpub-github-config', { crate: crate.name });
      assert.strictEqual(configs.length, 0);
    });

    test('returns an error if the user is not authenticated', async function (assert) {
      let user = this.db.user.create();

      let crate = this.db.crate.create();
      this.db.version.create({ crate });
      this.db.crateOwnership.create({ crate, user });

      // Create a config in the database that will be queried later
      this.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      this.authenticateAs(user);
      let configs = await this.store.query('trustpub-github-config', { crate: crate.name });
      assert.strictEqual(configs.length, 1);

      db.mswSession.deleteMany({});

      await assert.rejects(configs[0].destroyRecord(), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'must be logged in to perform that action' }]);
        return true;
      });
    });

    test('returns an error if the user is not an owner of the crate', async function (assert) {
      let user1 = this.db.user.create();
      let user2 = this.db.user.create();

      let crate = this.db.crate.create();
      this.db.version.create({ crate });
      this.db.crateOwnership.create({ crate, user: user1 });

      // Create a config in the database that will be queried later
      this.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      this.authenticateAs(user1);
      let configs = await this.store.query('trustpub-github-config', { crate: crate.name });
      assert.strictEqual(configs.length, 1);

      db.mswSession.deleteMany({});
      this.authenticateAs(user2);

      await assert.rejects(configs[0].destroyRecord(), function (error) {
        assert.deepEqual(error.errors, [{ detail: 'You are not an owner of this crate' }]);
        return true;
      });
    });
  });
});
