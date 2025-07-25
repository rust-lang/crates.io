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
    let user = context.db.user.create({
      emails: [context.db.email.create({ email: 'user-1@crates.io', verified: true, primary: true })],
    });

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
    assert.dom('[data-test-repository-owner]').exists();
    assert.dom('[data-test-repository-name]').exists();
    assert.dom('[data-test-workflow-filename]').exists();
    assert.dom('[data-test-environment]').exists();
    assert.dom('[data-test-add]').exists();
    assert.dom('[data-test-cancel]').exists();

    // Fill in the form
    await fillIn('[data-test-repository-owner]', 'rust-lang');
    await fillIn('[data-test-repository-name]', 'crates.io');
    await fillIn('[data-test-workflow-filename]', 'ci.yml');
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
    assert.dom('[data-test-repository-owner-group] [data-test-error]').exists();
    assert.dom('[data-test-repository-name-group] [data-test-error]').exists();
    assert.dom('[data-test-workflow-filename-group] [data-test-error]').exists();

    // Fill in the required fields
    await fillIn('[data-test-repository-owner]', 'rust-lang');
    await fillIn('[data-test-repository-name]', 'crates.io');
    await fillIn('[data-test-workflow-filename]', 'ci.yml');

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
    await fillIn('[data-test-repository-owner]', 'rust-lang');
    await fillIn('[data-test-repository-name]', 'crates.io');
    await fillIn('[data-test-workflow-filename]', 'ci.yml');

    // Submit the form
    let clickPromise = click('[data-test-add]');
    await waitFor('[data-test-add] [data-test-spinner]');
    assert.dom('[data-test-publisher]').isDisabled();
    assert.dom('[data-test-repository-owner]').isDisabled();
    assert.dom('[data-test-repository-name]').isDisabled();
    assert.dom('[data-test-workflow-filename]').isDisabled();
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
    assert.dom('[data-test-repository-owner]').isEnabled();
    assert.dom('[data-test-repository-name]').isEnabled();
    assert.dom('[data-test-workflow-filename]').isEnabled();
    assert.dom('[data-test-environment]').isEnabled();
    assert.dom('[data-test-add]').isEnabled();

    await click('[data-test-cancel]');
    assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
    assert.dom('[data-test-github-config]').exists({ count: 0 });
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
        owner: 'rust-lang',
        repo: 'crates.io',
      },
      {
        name: 'with .git suffix',
        url: 'https://github.com/rust-lang/crates.io.git',
        owner: 'rust-lang',
        repo: 'crates.io',
      },
      {
        name: 'with extra path segments',
        url: 'https://github.com/Byron/google-apis-rs/tree/main/gen/privateca1',
        owner: 'Byron',
        repo: 'google-apis-rs',
      },
      {
        name: 'non-github url',
        url: 'https://gitlab.com/rust-lang/crates.io',
        owner: '',
        repo: '',
      },
      {
        name: 'not a url',
        url: 'not a url',
        owner: '',
        repo: '',
      },
      {
        name: 'empty string',
        url: '',
        owner: '',
        repo: '',
      },
      {
        name: 'null',
        url: null,
        owner: '',
        repo: '',
      },
    ];

    for (let { name, url, owner, repo } of testCases) {
      test(name, async function (assert) {
        let { crate } = prepare(this);
        this.db.crate.update({
          where: { id: { equals: crate.id } },
          data: { repository: url },
        });

        await visit(`/crates/${crate.name}/settings/new-trusted-publisher`);

        assert.dom('[data-test-repository-owner]').hasValue(owner);
        assert.dom('[data-test-repository-name]').hasValue(repo);
      });
    }
  });
});
