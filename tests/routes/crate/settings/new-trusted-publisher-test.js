import { click, currentURL, fillIn, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import percySnapshot from '@percy/ember';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../../helpers/visit-ignoring-abort';

module('Route | crate.settings.new-trusted-publisher', hooks => {
  setupApplicationTest(hooks);

  function prepare(context) {
    let user = context.db.user.create();

    let crate = context.db.crate.create({ name: 'foo' });
    context.db.version.create({ crate });
    context.db.crateOwnership.create({ crate, user });

    context.authenticateAs(user);

    return { crate, user };
  }

  test('unauthenticated', async function (assert) {
    let { crate } = prepare(this);

    this.db.mswSession.deleteMany({});

    await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
    assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('not an owner', async function (assert) {
    let { crate } = prepare(this);

    this.db.crateOwnership.deleteMany({});

    await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
    assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);
    assert.dom('[data-test-title]').hasText('This page is only accessible by crate owners');
    assert.dom('[data-test-go-back]').exists();
  });

  test('cancel button', async function (assert) {
    let { crate } = prepare(this);

    await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
    assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

    // Click the cancel button
    await click('[data-test-cancel]');

    // Check that we're redirected back to the crate settings page
    assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
  });

  module('prefill', function () {
    let testCases = [
      {
        name: 'simple https',
        url: 'https://github.com/rust-lang/crates.io',
        publisher: 'GitHub',
        owner: 'rust-lang',
        repo: 'crates.io',
      },
      {
        name: 'with .git suffix',
        url: 'https://github.com/rust-lang/crates.io.git',
        publisher: 'GitHub',
        owner: 'rust-lang',
        repo: 'crates.io',
      },
      {
        name: 'with extra path segments',
        url: 'https://github.com/Byron/google-apis-rs/tree/main/gen/privateca1',
        publisher: 'GitHub',
        owner: 'Byron',
        repo: 'google-apis-rs',
      },
      {
        name: 'non-github url',
        url: 'https://gitlab.com/rust-lang/crates.io',
        publisher: 'GitHub',
        owner: '',
        repo: '',
      },
      {
        name: 'not a url',
        url: 'not a url',
        publisher: 'GitHub',
        owner: '',
        repo: '',
      },
      {
        name: 'empty string',
        url: '',
        publisher: 'GitHub',
        owner: '',
        repo: '',
      },
      {
        name: 'null',
        url: null,
        publisher: 'GitHub',
        owner: '',
        repo: '',
      },
    ];

    for (let { name, url, publisher, owner, repo } of testCases) {
      test(name, async function (assert) {
        let { crate } = prepare(this);
        this.db.crate.update({
          where: { id: { equals: crate.id } },
          data: { repository: url },
        });

        await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);

        assert.dom('[data-test-publisher]').hasValue(publisher);
        assert.dom('[data-test-namespace]').hasValue(owner);
        assert.dom('[data-test-project]').hasValue(repo);
      });
    }
  });

  module('GitHub', function () {
    test('happy path', async function (assert) {
      let { crate } = prepare(this);

      this.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'johndoe',
        repository_name: 'crates.io',
        workflow_filename: 'release.yml',
      });

      await visit(`/crates/${crate.name}/settings`);
      await click('[data-test-add-trusted-publisher-button]');
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

      await percySnapshot(assert);

      // Check that the form is displayed correctly
      assert.dom('[data-test-publisher]').exists();
      assert.dom('[data-test-namespace]').exists();
      assert.dom('[data-test-project]').exists();
      assert.dom('[data-test-workflow]').exists();
      assert.dom('[data-test-environment]').exists();
      assert.dom('[data-test-add]').exists();
      assert.dom('[data-test-cancel]').exists();

      // Fill in the form
      await fillIn('[data-test-namespace]', 'rust-lang');
      await fillIn('[data-test-project]', 'crates.io');
      await fillIn('[data-test-workflow]', 'ci.yml');
      await fillIn('[data-test-environment]', 'release');

      // Submit the form
      await click('[data-test-add]');

      // Check that we're redirected back to the crate settings page
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);

      // Check that the config was created
      let config = this.db.trustpubGithubConfig.findFirst({
        where: {
          repository_owner: { equals: 'rust-lang' },
          repository_name: { equals: 'crates.io' },
          workflow_filename: { equals: 'ci.yml' },
          environment: { equals: 'release' },
        },
      });
      assert.ok(config, 'Config was created');

      // Check that the success notification is displayed
      assert.dom('[data-test-notification-message]').hasText('Trusted Publishing configuration added successfully');

      // Check that the config is displayed on the crate settings page
      assert.dom('[data-test-github-config]').exists({ count: 2 });
      assert.dom('[data-test-github-config="2"] td:nth-child(1)').hasText('GitHub');
      assert.dom('[data-test-github-config="2"] td:nth-child(2)').includesText('Repository: rust-lang/crates.io');
      assert.dom('[data-test-github-config="2"] td:nth-child(2)').includesText('Workflow: ci.yml');
      assert.dom('[data-test-github-config="2"] td:nth-child(2)').includesText('Environment: release');
    });

    test('validation errors', async function (assert) {
      let { crate } = prepare(this);

      await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

      // Submit the form without filling in required fields
      await click('[data-test-add]');

      // Check that validation errors are displayed
      assert.dom('[data-test-namespace-group] [data-test-error]').exists();
      assert.dom('[data-test-project-group] [data-test-error]').exists();
      assert.dom('[data-test-workflow-group] [data-test-error]').exists();

      // Fill in the required fields
      await fillIn('[data-test-namespace]', 'rust-lang');
      await fillIn('[data-test-project]', 'crates.io');
      await fillIn('[data-test-workflow]', 'ci.yml');

      // Submit the form
      await click('[data-test-add]');

      // Check that we're redirected back to the crate settings page
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
    });

    test('loading and error state', async function (assert) {
      let { crate } = prepare(this);

      // Mock the server to return an error
      let deferred = defer();
      this.worker.use(http.post('/api/v1/trusted_publishing/github_configs', () => deferred.promise));

      await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

      // Fill in the form
      await fillIn('[data-test-namespace]', 'rust-lang');
      await fillIn('[data-test-project]', 'crates.io');
      await fillIn('[data-test-workflow]', 'ci.yml');

      // Submit the form
      let clickPromise = click('[data-test-add]');
      await waitFor('[data-test-add] [data-test-spinner]');
      assert.dom('[data-test-publisher]').isDisabled();
      assert.dom('[data-test-namespace]').isDisabled();
      assert.dom('[data-test-project]').isDisabled();
      assert.dom('[data-test-workflow]').isDisabled();
      assert.dom('[data-test-environment]').isDisabled();
      assert.dom('[data-test-add]').isDisabled();

      // Resolve the deferred with an error
      deferred.resolve(HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 }));
      await clickPromise;

      // Check that the error notification is displayed
      assert
        .dom('[data-test-notification-message]')
        .hasText('An error has occurred while adding the Trusted Publishing configuration: Server error');

      assert.dom('[data-test-publisher]').isEnabled();
      assert.dom('[data-test-namespace]').isEnabled();
      assert.dom('[data-test-project]').isEnabled();
      assert.dom('[data-test-workflow]').isEnabled();
      assert.dom('[data-test-environment]').isEnabled();
      assert.dom('[data-test-add]').isEnabled();

      await click('[data-test-cancel]');
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
      assert.dom('[data-test-github-config]').exists({ count: 0 });
    });

    module('workflow verification', function () {
      test('success case (200 OK)', async function (assert) {
        let { crate } = prepare(this);

        await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
        assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

        this.worker.use(
          http.head('https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/ci.yml', () => {
            return new HttpResponse(null, { status: 200 });
          }),
        );

        assert
          .dom('[data-test-workflow-verification="initial"]')
          .hasText('The workflow filename will be verified once all necessary fields are filled.');

        await fillIn('[data-test-namespace]', 'rust-lang');
        await fillIn('[data-test-project]', 'crates.io');
        await fillIn('[data-test-workflow]', 'ci.yml');

        await waitFor('[data-test-workflow-verification="success"]');

        let expected =
          '✓ Workflow file found at https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/ci.yml';
        assert.dom('[data-test-workflow-verification="success"]').hasText(expected);
      });

      test('not found case (404)', async function (assert) {
        let { crate } = prepare(this);

        await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
        assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

        this.worker.use(
          http.head('https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/missing.yml', () => {
            return new HttpResponse(null, { status: 404 });
          }),
        );

        await fillIn('[data-test-namespace]', 'rust-lang');
        await fillIn('[data-test-project]', 'crates.io');
        await fillIn('[data-test-workflow]', 'missing.yml');

        await waitFor('[data-test-workflow-verification="not-found"]');

        let expected =
          '⚠ Workflow file not found at https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/missing.yml';
        assert.dom('[data-test-workflow-verification="not-found"]').hasText(expected);

        // Verify form can still be submitted
        await click('[data-test-add]');
        assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
      });

      test('server error (5xx)', async function (assert) {
        let { crate } = prepare(this);

        await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
        assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

        this.worker.use(
          http.head('https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/ci.yml', () => {
            return new HttpResponse(null, { status: 500 });
          }),
        );

        await fillIn('[data-test-namespace]', 'rust-lang');
        await fillIn('[data-test-project]', 'crates.io');
        await fillIn('[data-test-workflow]', 'ci.yml');

        await waitFor('[data-test-workflow-verification="error"]');

        let expected =
          '⚠ Could not verify workflow file at https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/ci.yml (network error)';
        assert.dom('[data-test-workflow-verification="error"]').hasText(expected);
      });
    });
  });

  module('GitLab', function () {
    test('happy path', async function (assert) {
      let { crate } = prepare(this);

      this.db.trustpubGitlabConfig.create({
        crate,
        namespace: 'johndoe',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
      });

      await visit(`/crates/${crate.name}/settings`);
      await click('[data-test-add-trusted-publisher-button]');
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

      // Check that the form is displayed correctly
      assert.dom('[data-test-publisher]').exists();
      assert.dom('[data-test-namespace]').exists();
      assert.dom('[data-test-project]').exists();
      assert.dom('[data-test-workflow]').exists();

      // Select GitLab from the publisher dropdown
      await fillIn('[data-test-publisher]', 'GitLab');

      // Check that GitLab fields are displayed
      assert.dom('[data-test-namespace]').exists();
      assert.dom('[data-test-project]').exists();
      assert.dom('[data-test-workflow]').exists();
      assert.dom('[data-test-environment]').exists();
      assert.dom('[data-test-add]').exists();
      assert.dom('[data-test-cancel]').exists();

      // Fill in the form
      await fillIn('[data-test-namespace]', 'rust-lang');
      await fillIn('[data-test-project]', 'crates.io');
      await fillIn('[data-test-workflow]', '.gitlab-ci.yml');
      await fillIn('[data-test-environment]', 'production');

      // Submit the form
      await click('[data-test-add]');

      // Check that we're redirected back to the crate settings page
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);

      // Check that the config was created
      let config = this.db.trustpubGitlabConfig.findFirst({
        where: {
          namespace: { equals: 'rust-lang' },
          project: { equals: 'crates.io' },
          workflow_filepath: { equals: '.gitlab-ci.yml' },
          environment: { equals: 'production' },
        },
      });
      assert.ok(config, 'Config was created');

      // Check that the success notification is displayed
      assert.dom('[data-test-notification-message]').hasText('Trusted Publishing configuration added successfully');

      // Check that the config is displayed on the crate settings page
      assert.dom('[data-test-gitlab-config]').exists({ count: 2 });
      assert.dom('[data-test-gitlab-config="2"] td:nth-child(1)').hasText('GitLab');
      assert.dom('[data-test-gitlab-config="2"] td:nth-child(2)').includesText('Repository: rust-lang/crates.io');
      assert.dom('[data-test-gitlab-config="2"] td:nth-child(2)').includesText('Workflow: .gitlab-ci.yml');
      assert.dom('[data-test-gitlab-config="2"] td:nth-child(2)').includesText('Environment: production');
    });

    test('validation errors', async function (assert) {
      let { crate } = prepare(this);

      await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

      // Select GitLab from the publisher dropdown
      await fillIn('[data-test-publisher]', 'GitLab');

      // Submit the form without filling in required fields
      await click('[data-test-add]');

      // Check that validation errors are displayed
      assert.dom('[data-test-namespace-group] [data-test-error]').exists();
      assert.dom('[data-test-project-group] [data-test-error]').exists();
      assert.dom('[data-test-workflow-group] [data-test-error]').exists();

      // Fill in the required fields
      await fillIn('[data-test-namespace]', 'rust-lang');
      await fillIn('[data-test-project]', 'crates.io');
      await fillIn('[data-test-workflow]', '.gitlab-ci.yml');

      // Submit the form
      await click('[data-test-add]');

      // Check that we're redirected back to the crate settings page
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
    });

    test('loading and error state', async function (assert) {
      let { crate } = prepare(this);

      // Mock the server to return an error
      let deferred = defer();
      this.worker.use(http.post('/api/v1/trusted_publishing/gitlab_configs', () => deferred.promise));

      await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings/new-trusted-publisher`);

      // Select GitLab from the publisher dropdown
      await fillIn('[data-test-publisher]', 'GitLab');

      // Fill in the form
      await fillIn('[data-test-namespace]', 'rust-lang');
      await fillIn('[data-test-project]', 'crates.io');
      await fillIn('[data-test-workflow]', '.gitlab-ci.yml');

      // Submit the form
      let clickPromise = click('[data-test-add]');
      await waitFor('[data-test-add] [data-test-spinner]');
      assert.dom('[data-test-publisher]').isDisabled();
      assert.dom('[data-test-namespace]').isDisabled();
      assert.dom('[data-test-project]').isDisabled();
      assert.dom('[data-test-workflow]').isDisabled();
      assert.dom('[data-test-environment]').isDisabled();
      assert.dom('[data-test-add]').isDisabled();

      // Resolve the deferred with an error
      deferred.resolve(HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 }));
      await clickPromise;

      // Check that the error notification is displayed
      assert
        .dom('[data-test-notification-message]')
        .hasText('An error has occurred while adding the Trusted Publishing configuration: Server error');

      assert.dom('[data-test-publisher]').isEnabled();
      assert.dom('[data-test-namespace]').isEnabled();
      assert.dom('[data-test-project]').isEnabled();
      assert.dom('[data-test-workflow]').isEnabled();
      assert.dom('[data-test-environment]').isEnabled();
      assert.dom('[data-test-add]').isEnabled();

      await click('[data-test-cancel]');
      assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
      assert.dom('[data-test-gitlab-config]').exists({ count: 0 });
    });
  });
});
