import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';
import { defer } from '@/e2e/deferred';

test.describe('Route | crate.settings.new-trusted-publisher', { tag: '@routes' }, () => {
  async function prepare(msw) {
    let user = msw.db.user.create();

    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate });
    msw.db.crateOwnership.create({ crate, user });

    await msw.authenticateAs(user);

    return { crate, user };
  }

  test('unauthenticated', async ({ msw, page }) => {
    let { crate } = await prepare(msw);

    msw.db.mswSession.deleteMany({});

    await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
    await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });

  test('not an owner', async ({ msw, page }) => {
    let { crate } = await prepare(msw);

    msw.db.crateOwnership.deleteMany({});

    await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
    await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);
    await expect(page.locator('[data-test-title]')).toHaveText('This page is only accessible by crate owners');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
  });

  test('cancel button', async ({ msw, page }) => {
    let { crate } = await prepare(msw);

    await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
    await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

    // Click the cancel button
    await page.click('[data-test-cancel]');

    // Check that we're redirected back to the crate settings page
    await expect(page).toHaveURL(`/crates/${crate.name}/settings`);
  });

  test.describe('prefill', () => {
    const testCases = [
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

    for (const { name, url, publisher, owner, repo } of testCases) {
      test(name, async ({ msw, page }) => {
        let { crate } = await prepare(msw);

        msw.db.crate.update({
          where: { id: { equals: crate.id } },
          data: { repository: url },
        });

        await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);

        await expect(page.locator('[data-test-publisher]')).toHaveValue(publisher);
        await expect(page.locator('[data-test-namespace]')).toHaveValue(owner);
        await expect(page.locator('[data-test-project]')).toHaveValue(repo);
      });
    }
  });

  test.describe('GitHub', () => {
    test('happy path', async ({ msw, page, percy }) => {
      let { crate } = await prepare(msw);

      msw.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'johndoe',
        repository_name: 'crates.io',
        workflow_filename: 'release.yml',
      });

      await page.goto(`/crates/${crate.name}/settings`);
      await page.click('[data-test-add-trusted-publisher-button]');
      await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

      await percy.snapshot();

      // Check that the form is displayed correctly
      await expect(page.locator('[data-test-publisher]')).toBeVisible();
      await expect(page.locator('[data-test-namespace]')).toBeVisible();
      await expect(page.locator('[data-test-project]')).toBeVisible();
      await expect(page.locator('[data-test-workflow]')).toBeVisible();
      await expect(page.locator('[data-test-environment]')).toBeVisible();
      await expect(page.locator('[data-test-add]')).toBeVisible();
      await expect(page.locator('[data-test-cancel]')).toBeVisible();

      // Fill in the form
      await page.fill('[data-test-namespace]', 'rust-lang');
      await page.fill('[data-test-project]', 'crates.io');
      await page.fill('[data-test-workflow]', 'ci.yml');
      await page.fill('[data-test-environment]', 'release');

      // Submit the form
      await page.click('[data-test-add]');

      // Check that we're redirected back to the crate settings page
      await expect(page).toHaveURL(`/crates/${crate.name}/settings`);

      // Check that the config was created
      let config = msw.db.trustpubGithubConfig.findFirst({
        where: {
          repository_owner: { equals: 'rust-lang' },
          repository_name: { equals: 'crates.io' },
          workflow_filename: { equals: 'ci.yml' },
          environment: { equals: 'release' },
        },
      });
      expect(config, 'Config was created').toBeDefined();

      // Check that the success notification is displayed
      await expect(page.locator('[data-test-notification-message]')).toHaveText(
        'Trusted Publishing configuration added successfully',
      );

      // Check that the config is displayed on the crate settings page
      await expect(page.locator('[data-test-github-config]')).toHaveCount(2);
      await expect(page.locator('[data-test-github-config="2"] td:nth-child(1)')).toHaveText('GitHub');
      let details = page.locator('[data-test-github-config="2"] td:nth-child(2)');
      await expect(details).toContainText('Repository: rust-lang/crates.io');
      await expect(details).toContainText('Workflow: ci.yml');
      await expect(details).toContainText('Environment: release');
    });

    test('validation errors', async ({ msw, page }) => {
      let { crate } = await prepare(msw);

      await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
      await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

      // Submit the form without filling in required fields
      await page.click('[data-test-add]');

      // Check that validation errors are displayed
      await expect(page.locator('[data-test-namespace-group] [data-test-error]')).toBeVisible();
      await expect(page.locator('[data-test-project-group] [data-test-error]')).toBeVisible();
      await expect(page.locator('[data-test-workflow-group] [data-test-error]')).toBeVisible();

      // Fill in the required fields
      await page.fill('[data-test-namespace]', 'rust-lang');
      await page.fill('[data-test-project]', 'crates.io');
      await page.fill('[data-test-workflow]', 'ci.yml');

      // Submit the form
      await page.click('[data-test-add]');

      // Check that we're redirected back to the crate settings page
      await expect(page).toHaveURL(`/crates/${crate.name}/settings`);
    });

    test('loading and error state', async ({ msw, page }) => {
      let { crate } = await prepare(msw);

      // Mock the server to return an error
      let deferred = defer();
      msw.worker.use(http.post('/api/v1/trusted_publishing/github_configs', () => deferred.promise));

      await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
      await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

      // Fill in the form
      await page.fill('[data-test-namespace]', 'rust-lang');
      await page.fill('[data-test-project]', 'crates.io');
      await page.fill('[data-test-workflow]', 'ci.yml');

      // Submit the form
      await page.click('[data-test-add]');
      await expect(page.locator('[data-test-add] [data-test-spinner]')).toBeVisible();
      await expect(page.locator('[data-test-publisher]')).toBeDisabled();
      await expect(page.locator('[data-test-namespace]')).toBeDisabled();
      await expect(page.locator('[data-test-project]')).toBeDisabled();
      await expect(page.locator('[data-test-workflow]')).toBeDisabled();
      await expect(page.locator('[data-test-environment]')).toBeDisabled();
      await expect(page.locator('[data-test-add]')).toBeDisabled();

      // Resolve the deferred with an error
      deferred.resolve(HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 }));

      // Check that the error notification is displayed
      await expect(page.locator('[data-test-notification-message]')).toHaveText(
        'An error has occurred while adding the Trusted Publishing configuration: Server error',
      );

      await expect(page.locator('[data-test-publisher]')).toBeEnabled();
      await expect(page.locator('[data-test-namespace]')).toBeEnabled();
      await expect(page.locator('[data-test-project]')).toBeEnabled();
      await expect(page.locator('[data-test-workflow]')).toBeEnabled();
      await expect(page.locator('[data-test-environment]')).toBeEnabled();
      await expect(page.locator('[data-test-add]')).toBeEnabled();

      await page.click('[data-test-cancel]');
      await expect(page).toHaveURL(`/crates/${crate.name}/settings`);
      await expect(page.locator('[data-test-github-config]')).toHaveCount(0);
    });

    test.describe('workflow verification', () => {
      test('success case (200 OK)', async ({ msw, page }) => {
        let { crate } = await prepare(msw);

        await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
        await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

        await msw.worker.use(
          http.head('https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/ci.yml', () => {
            return new HttpResponse(null, { status: 200 });
          }),
        );

        await expect(page.locator('[data-test-workflow-verification="initial"]')).toHaveText(
          'The workflow filename will be verified once all necessary fields are filled.',
        );

        await page.fill('[data-test-namespace]', 'rust-lang');
        await page.fill('[data-test-project]', 'crates.io');
        await page.fill('[data-test-workflow]', 'ci.yml');

        await expect(page.locator('[data-test-workflow-verification="success"]')).toHaveText(
          '✓ Workflow file found at https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/ci.yml',
        );
      });

      test('not found case (404)', async ({ msw, page }) => {
        let { crate } = await prepare(msw);

        await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
        await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

        await msw.worker.use(
          http.head('https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/missing.yml', () => {
            return new HttpResponse(null, { status: 404 });
          }),
        );

        await page.fill('[data-test-namespace]', 'rust-lang');
        await page.fill('[data-test-project]', 'crates.io');
        await page.fill('[data-test-workflow]', 'missing.yml');

        await expect(page.locator('[data-test-workflow-verification="not-found"]')).toHaveText(
          '⚠ Workflow file not found at https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/missing.yml',
        );

        // Verify form can still be submitted
        await page.click('[data-test-add]');
        await expect(page).toHaveURL(`/crates/${crate.name}/settings`);
      });

      test('server error (5xx)', async ({ msw, page }) => {
        let { crate } = await prepare(msw);

        await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
        await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

        await msw.worker.use(
          http.head('https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/ci.yml', () => {
            return new HttpResponse(null, { status: 500 });
          }),
        );

        await page.fill('[data-test-namespace]', 'rust-lang');
        await page.fill('[data-test-project]', 'crates.io');
        await page.fill('[data-test-workflow]', 'ci.yml');

        await expect(page.locator('[data-test-workflow-verification="error"]')).toHaveText(
          '⚠ Could not verify workflow file at https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/ci.yml (network error)',
        );
      });
    });
  });

  test.describe('GitLab', () => {
    test('happy path', async ({ msw, page }) => {
      let { crate } = await prepare(msw);

      msw.db.trustpubGitlabConfig.create({
        crate,
        namespace: 'johndoe',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
      });

      await page.goto(`/crates/${crate.name}/settings`);
      await page.click('[data-test-add-trusted-publisher-button]');
      await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

      // Check that the form is displayed correctly
      await expect(page.locator('[data-test-publisher]')).toBeVisible();
      await expect(page.locator('[data-test-namespace]')).toBeVisible();
      await expect(page.locator('[data-test-project]')).toBeVisible();
      await expect(page.locator('[data-test-workflow]')).toBeVisible();

      // Select GitLab from the publisher dropdown
      await page.selectOption('[data-test-publisher]', 'GitLab');

      // Check that GitLab fields are displayed
      await expect(page.locator('[data-test-namespace]')).toBeVisible();
      await expect(page.locator('[data-test-project]')).toBeVisible();
      await expect(page.locator('[data-test-workflow]')).toBeVisible();
      await expect(page.locator('[data-test-environment]')).toBeVisible();
      await expect(page.locator('[data-test-add]')).toBeVisible();
      await expect(page.locator('[data-test-cancel]')).toBeVisible();

      // Fill in the form
      await page.fill('[data-test-namespace]', 'rust-lang');
      await page.fill('[data-test-project]', 'crates.io');
      await page.fill('[data-test-workflow]', '.gitlab-ci.yml');
      await page.fill('[data-test-environment]', 'production');

      // Submit the form
      await page.click('[data-test-add]');

      // Check that we're redirected back to the crate settings page
      await expect(page).toHaveURL(`/crates/${crate.name}/settings`);

      // Check that the config was created
      let config = msw.db.trustpubGitlabConfig.findFirst({
        where: {
          namespace: { equals: 'rust-lang' },
          project: { equals: 'crates.io' },
          workflow_filepath: { equals: '.gitlab-ci.yml' },
          environment: { equals: 'production' },
        },
      });
      expect(config, 'Config was created').toBeDefined();

      // Check that the success notification is displayed
      await expect(page.locator('[data-test-notification-message]')).toHaveText(
        'Trusted Publishing configuration added successfully',
      );

      // Check that the config is displayed on the crate settings page
      await expect(page.locator('[data-test-gitlab-config]')).toHaveCount(2);
      await expect(page.locator('[data-test-gitlab-config="2"] td:nth-child(1)')).toHaveText('GitLab');
      let details = page.locator('[data-test-gitlab-config="2"] td:nth-child(2)');
      await expect(details).toContainText('Repository: rust-lang/crates.io');
      await expect(details).toContainText('Workflow: .gitlab-ci.yml');
      await expect(details).toContainText('Environment: production');
    });

    test('validation errors', async ({ msw, page }) => {
      let { crate } = await prepare(msw);

      await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
      await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

      // Select GitLab from the publisher dropdown
      await page.selectOption('[data-test-publisher]', 'GitLab');

      // Submit the form without filling in required fields
      await page.click('[data-test-add]');

      // Check that validation errors are displayed
      await expect(page.locator('[data-test-namespace-group] [data-test-error]')).toBeVisible();
      await expect(page.locator('[data-test-project-group] [data-test-error]')).toBeVisible();
      await expect(page.locator('[data-test-workflow-group] [data-test-error]')).toBeVisible();

      // Fill in the required fields
      await page.fill('[data-test-namespace]', 'rust-lang');
      await page.fill('[data-test-project]', 'crates.io');
      await page.fill('[data-test-workflow]', '.gitlab-ci.yml');

      // Submit the form
      await page.click('[data-test-add]');

      // Check that we're redirected back to the crate settings page
      await expect(page).toHaveURL(`/crates/${crate.name}/settings`);
    });

    test('loading and error state', async ({ msw, page }) => {
      let { crate } = await prepare(msw);

      // Mock the server to return an error
      let deferred = defer();
      msw.worker.use(http.post('/api/v1/trusted_publishing/gitlab_configs', () => deferred.promise));

      await page.goto(`/crates/${crate.name}/settings/new-trusted-publisher`);
      await expect(page).toHaveURL(`/crates/${crate.name}/settings/new-trusted-publisher`);

      // Select GitLab from the publisher dropdown
      await page.selectOption('[data-test-publisher]', 'GitLab');

      // Fill in the form
      await page.fill('[data-test-namespace]', 'rust-lang');
      await page.fill('[data-test-project]', 'crates.io');
      await page.fill('[data-test-workflow]', '.gitlab-ci.yml');

      // Submit the form
      await page.click('[data-test-add]');
      await expect(page.locator('[data-test-add] [data-test-spinner]')).toBeVisible();
      await expect(page.locator('[data-test-publisher]')).toBeDisabled();
      await expect(page.locator('[data-test-namespace]')).toBeDisabled();
      await expect(page.locator('[data-test-project]')).toBeDisabled();
      await expect(page.locator('[data-test-workflow]')).toBeDisabled();
      await expect(page.locator('[data-test-environment]')).toBeDisabled();
      await expect(page.locator('[data-test-add]')).toBeDisabled();

      // Resolve the deferred with an error
      deferred.resolve(HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 }));

      // Check that the error notification is displayed
      await expect(page.locator('[data-test-notification-message]')).toHaveText(
        'An error has occurred while adding the Trusted Publishing configuration: Server error',
      );

      await expect(page.locator('[data-test-publisher]')).toBeEnabled();
      await expect(page.locator('[data-test-namespace]')).toBeEnabled();
      await expect(page.locator('[data-test-project]')).toBeEnabled();
      await expect(page.locator('[data-test-workflow]')).toBeEnabled();
      await expect(page.locator('[data-test-environment]')).toBeEnabled();
      await expect(page.locator('[data-test-add]')).toBeEnabled();

      await page.click('[data-test-cancel]');
      await expect(page).toHaveURL(`/crates/${crate.name}/settings`);
      await expect(page.locator('[data-test-gitlab-config]')).toHaveCount(0);
    });
  });
});
