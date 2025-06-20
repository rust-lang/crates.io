import { click, currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../helpers/visit-ignoring-abort';

module('Route | crate.settings', hooks => {
  setupApplicationTest(hooks);

  function prepare(context) {
    const user = context.db.user.create();

    const crate = context.db.crate.create({ name: 'foo' });
    context.db.version.create({ crate });
    context.db.crateOwnership.create({ crate, user });

    return { crate, user };
  }

  test('unauthenticated', async function (assert) {
    const crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate });

    await visit('/crates/foo/settings');
    assert.strictEqual(currentURL(), '/crates/foo/settings');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('not an owner', async function (assert) {
    const { crate } = prepare(this);

    const otherUser = this.db.user.create();
    this.authenticateAs(otherUser);

    await visit(`/crates/${crate.name}/settings`);
    assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
    assert.dom('[data-test-title]').hasText('This page is only accessible by crate owners');
    assert.dom('[data-test-go-back]').exists();
  });

  test('happy path', async function (assert) {
    const { crate, user } = prepare(this);
    this.authenticateAs(user);

    await visit(`/crates/${crate.name}/settings`);
    assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);

    assert.dom('[data-test-add-owner-button]').exists();
    assert.dom('[data-test-owners]').exists();
    assert.dom(`[data-test-owner-user="${user.login}"]`).exists();
    assert.dom('[data-test-remove-owner-button]').exists();

    // Disabled for now, until we make this feature publicly available
    // assert.dom('[data-test-trusted-publishing]').exists();
    // assert.dom('[data-test-no-config]').exists();
    // assert.dom('[data-test-github-config]').doesNotExist();
    assert.dom('[data-test-trusted-publishing]').doesNotExist();

    assert.dom('[data-test-delete-button]').exists();
  });

  module('Trusted Publishing', function () {
    test('happy path', async function (assert) {
      const { crate, user } = prepare(this);
      this.authenticateAs(user);

      // Create two GitHub configs for the crate
      this.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      this.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'johndoe',
        repository_name: 'crates.io',
        workflow_filename: 'release.yml',
        environment: 'release',
      });

      await visit(`/crates/${crate.name}/settings`);
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);

      await percySnapshot(assert);

      // Check that the GitHub config is displayed
      assert.dom('[data-test-trusted-publishing]').exists();
      assert.dom('[data-test-github-config]').exists({ count: 2 });
      assert.dom('[data-test-github-config="1"] td:nth-child(1)').hasText('GitHub');
      assert.dom('[data-test-github-config="1"] td:nth-child(2)').includesText('Repository: rust-lang/crates.io');
      assert.dom('[data-test-github-config="1"] td:nth-child(2)').includesText('Workflow: ci.yml');
      assert.dom('[data-test-github-config="1"] td:nth-child(2)').doesNotIncludeText('Environment');
      assert.dom('[data-test-github-config="1"] [data-test-remove-config-button]').exists();
      assert.dom('[data-test-github-config="2"] td:nth-child(1)').hasText('GitHub');
      assert.dom('[data-test-github-config="2"] td:nth-child(2)').includesText('Repository: johndoe/crates.io');
      assert.dom('[data-test-github-config="2"] td:nth-child(2)').includesText('Workflow: release.yml');
      assert.dom('[data-test-github-config="2"] td:nth-child(2)').includesText('Environment: release');
      assert.dom('[data-test-github-config="2"] [data-test-remove-config-button]').exists();
      assert.dom('[data-test-no-config]').doesNotExist();

      // Click the remove button
      await click('[data-test-github-config="2"] [data-test-remove-config-button]');

      // Check that the config is no longer displayed
      assert.dom('[data-test-github-config]').exists({ count: 1 });
      assert.dom('[data-test-github-config="1"] td:nth-child(2)').includesText('Repository: rust-lang/crates.io');
      assert.dom('[data-test-notification-message]').hasText('Trusted Publishing configuration removed successfully');
    });

    test('deletion failure', async function (assert) {
      let { crate, user } = prepare(this);
      this.authenticateAs(user);

      // Create a GitHub config for the crate
      let config = this.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
        environment: 'release',
      });

      // Mock the server to return an error when trying to delete the config
      this.worker.use(
        http.delete(`/api/v1/trusted_publishing/github_configs/${config.id}`, () => {
          return HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 });
        }),
      );

      await visit(`/crates/${crate.name}/settings`);
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
      assert.dom('[data-test-github-config]').exists({ count: 1 });

      await click('[data-test-remove-config-button]');
      assert.dom('[data-test-github-config]').exists({ count: 1 });
      assert
        .dom('[data-test-notification-message]')
        .hasText('Failed to remove Trusted Publishing configuration: Server error');
    });
  });
});
