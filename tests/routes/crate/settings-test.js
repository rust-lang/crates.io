import { click, currentURL, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

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

    assert.dom('[data-test-trusted-publishing]').exists();
    assert.dom('[data-test-no-config]').exists();
    assert.dom('[data-test-github-config]').doesNotExist();

    assert.dom('[data-test-delete-button]').exists();
  });

  module('Trusted Publishing', function () {
    test('mixed GitHub and GitLab configs', async function (assert) {
      const { crate, user } = prepare(this);
      this.authenticateAs(user);

      // Create GitHub configs
      this.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      // Create GitLab configs
      this.db.trustpubGitlabConfig.create({
        crate,
        namespace: 'johndoe',
        namespace_id: '1234',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
        environment: 'production',
      });

      await visit(`/crates/${crate.name}/settings`);
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);

      await percySnapshot(assert);

      // Check that both GitHub and GitLab configs are displayed
      assert.dom('[data-test-trusted-publishing]').exists();
      assert.dom('[data-test-github-config]').exists({ count: 1 });
      assert.dom('[data-test-gitlab-config]').exists({ count: 1 });

      // Verify GitHub config
      assert.dom('[data-test-github-config="1"] td:nth-child(1)').hasText('GitHub');
      assert.dom('[data-test-github-config="1"] td:nth-child(2)').includesText('Repository: rust-lang/crates.io');
      assert.dom('[data-test-github-config="1"] td:nth-child(2)').includesText('Workflow: ci.yml');

      // Verify GitLab config
      assert.dom('[data-test-gitlab-config="1"] td:nth-child(1)').hasText('GitLab');
      assert.dom('[data-test-gitlab-config="1"] td:nth-child(2)').includesText('Repository: johndoe/crates.io');
      assert.dom('[data-test-gitlab-config="1"] td:nth-child(2)').includesText('Namespace ID: 1234');
      assert.dom('[data-test-gitlab-config="1"] td:nth-child(2)').includesText('Workflow: .gitlab-ci.yml');
      assert.dom('[data-test-gitlab-config="1"] td:nth-child(2)').includesText('Environment: production');

      assert.dom('[data-test-no-config]').doesNotExist();
    });

    module('GitHub', function () {
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

    module('GitLab', function () {
      test('happy path', async function (assert) {
        const { crate, user } = prepare(this);
        this.authenticateAs(user);

        // Create two GitLab configs for the crate
        this.db.trustpubGitlabConfig.create({
          crate,
          namespace: 'rust-lang',
          project: 'crates.io',
          workflow_filepath: '.gitlab-ci.yml',
        });

        this.db.trustpubGitlabConfig.create({
          crate,
          namespace: 'johndoe',
          namespace_id: '1234',
          project: 'crates.io',
          workflow_filepath: '.gitlab-ci.yml',
          environment: 'release',
        });

        await visit(`/crates/${crate.name}/settings`);
        assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);

        // Check that the GitLab config is displayed
        assert.dom('[data-test-trusted-publishing]').exists();
        assert.dom('[data-test-gitlab-config]').exists({ count: 2 });
        assert.dom('[data-test-gitlab-config="1"] td:nth-child(1)').hasText('GitLab');
        assert.dom('[data-test-gitlab-config="1"] td:nth-child(2)').includesText('Repository: rust-lang/crates.io');
        assert.dom('[data-test-gitlab-config="1"] td:nth-child(2)').includesText('Namespace ID: (not yet set)');
        assert.dom('[data-test-gitlab-config="1"] td:nth-child(2)').includesText('Workflow: .gitlab-ci.yml');
        assert.dom('[data-test-gitlab-config="1"] td:nth-child(2)').doesNotIncludeText('Environment');
        assert.dom('[data-test-gitlab-config="1"] [data-test-remove-config-button]').exists();
        assert.dom('[data-test-gitlab-config="2"] td:nth-child(1)').hasText('GitLab');
        assert.dom('[data-test-gitlab-config="2"] td:nth-child(2)').includesText('Repository: johndoe/crates.io');
        assert.dom('[data-test-gitlab-config="2"] td:nth-child(2)').includesText('Namespace ID: 1234');
        assert.dom('[data-test-gitlab-config="2"] td:nth-child(2)').includesText('Workflow: .gitlab-ci.yml');
        assert.dom('[data-test-gitlab-config="2"] td:nth-child(2)').includesText('Environment: release');
        assert.dom('[data-test-gitlab-config="2"] [data-test-remove-config-button]').exists();
        assert.dom('[data-test-no-config]').doesNotExist();

        // Click the remove button
        await click('[data-test-gitlab-config="2"] [data-test-remove-config-button]');

        // Check that the config is no longer displayed
        assert.dom('[data-test-gitlab-config]').exists({ count: 1 });
        assert.dom('[data-test-gitlab-config="1"] td:nth-child(2)').includesText('Repository: rust-lang/crates.io');
        assert.dom('[data-test-notification-message]').hasText('Trusted Publishing configuration removed successfully');
      });

      test('deletion failure', async function (assert) {
        let { crate, user } = prepare(this);
        this.authenticateAs(user);

        // Create a GitLab config for the crate
        let config = this.db.trustpubGitlabConfig.create({
          crate,
          namespace: 'rust-lang',
          namespace_id: '1234',
          project: 'crates.io',
          workflow_filepath: '.gitlab-ci.yml',
          environment: 'release',
        });

        // Mock the server to return an error when trying to delete the config
        this.worker.use(
          http.delete(`/api/v1/trusted_publishing/gitlab_configs/${config.id}`, () => {
            return HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 });
          }),
        );

        await visit(`/crates/${crate.name}/settings`);
        assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
        assert.dom('[data-test-gitlab-config]').exists({ count: 1 });

        await click('[data-test-remove-config-button]');
        assert.dom('[data-test-gitlab-config]').exists({ count: 1 });
        assert
          .dom('[data-test-notification-message]')
          .hasText('Failed to remove Trusted Publishing configuration: Server error');
      });
    });
  });

  module('trustpub_only checkbox', function () {
    test('enabling trustpub_only', async function (assert) {
      const { crate, user } = prepare(this);
      this.authenticateAs(user);

      await visit(`/crates/${crate.name}/settings`);

      assert.dom('[data-test-trustpub-only-checkbox] [data-test-checkbox]').isNotChecked();
      assert.false(this.db.crate.findFirst({ where: { name: { equals: crate.name } } }).trustpubOnly);

      await click('[data-test-trustpub-only-checkbox] [data-test-checkbox]');

      assert.dom('[data-test-trustpub-only-checkbox] [data-test-checkbox]').isChecked();
      assert.true(this.db.crate.findFirst({ where: { name: { equals: crate.name } } }).trustpubOnly);
    });

    test('disabling trustpub_only', async function (assert) {
      const { crate, user } = prepare(this);
      this.db.crate.update({ where: { id: { equals: crate.id } }, data: { trustpubOnly: true } });
      this.authenticateAs(user);

      await visit(`/crates/${crate.name}/settings`);

      assert.dom('[data-test-trustpub-only-checkbox] [data-test-checkbox]').isChecked();
      assert.true(this.db.crate.findFirst({ where: { name: { equals: crate.name } } }).trustpubOnly);

      await click('[data-test-trustpub-only-checkbox] [data-test-checkbox]');

      assert.dom('[data-test-trustpub-only-checkbox] [data-test-checkbox]').isNotChecked();
      assert.false(this.db.crate.findFirst({ where: { name: { equals: crate.name } } }).trustpubOnly);
    });

    test('loading and error state', async function (assert) {
      const { crate, user } = prepare(this);
      this.authenticateAs(user);

      let deferred = defer();
      this.worker.use(http.patch('/api/v1/crates/:name', () => deferred.promise));

      await visit(`/crates/${crate.name}/settings`);

      assert.dom('[data-test-trustpub-only-checkbox] [data-test-checkbox]').exists();
      assert.dom('[data-test-trustpub-only-checkbox] [data-test-spinner]').doesNotExist();

      let clickPromise = click('[data-test-trustpub-only-checkbox] [data-test-checkbox]');

      await waitFor('[data-test-trustpub-only-checkbox] [data-test-spinner]');
      assert.dom('[data-test-trustpub-only-checkbox] [data-test-spinner]').exists();
      assert.dom('[data-test-trustpub-only-checkbox] [data-test-checkbox]').doesNotExist();

      deferred.resolve(HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 }));
      await clickPromise;

      assert.dom('[data-test-trustpub-only-checkbox] [data-test-checkbox]').exists();
      assert.dom('[data-test-trustpub-only-checkbox] [data-test-spinner]').doesNotExist();
      assert.dom('[data-test-notification-message]').hasText('Server error');
    });
  });
});
