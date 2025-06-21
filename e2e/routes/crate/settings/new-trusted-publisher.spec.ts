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
    await expect(page.locator('[data-test-repository-owner]')).toBeVisible();
    await expect(page.locator('[data-test-repository-name]')).toBeVisible();
    await expect(page.locator('[data-test-workflow-filename]')).toBeVisible();
    await expect(page.locator('[data-test-environment]')).toBeVisible();
    await expect(page.locator('[data-test-add]')).toBeVisible();
    await expect(page.locator('[data-test-cancel]')).toBeVisible();

    // Fill in the form
    await page.fill('[data-test-repository-owner]', 'rust-lang');
    await page.fill('[data-test-repository-name]', 'crates.io');
    await page.fill('[data-test-workflow-filename]', 'ci.yml');
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
    await expect(page.locator('[data-test-repository-owner-group] [data-test-error]')).toBeVisible();
    await expect(page.locator('[data-test-repository-name-group] [data-test-error]')).toBeVisible();
    await expect(page.locator('[data-test-workflow-filename-group] [data-test-error]')).toBeVisible();

    // Fill in the required fields
    await page.fill('[data-test-repository-owner]', 'rust-lang');
    await page.fill('[data-test-repository-name]', 'crates.io');
    await page.fill('[data-test-workflow-filename]', 'ci.yml');

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
    await page.fill('[data-test-repository-owner]', 'rust-lang');
    await page.fill('[data-test-repository-name]', 'crates.io');
    await page.fill('[data-test-workflow-filename]', 'ci.yml');

    // Submit the form
    await page.click('[data-test-add]');
    await expect(page.locator('[data-test-add] [data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-publisher]')).toBeDisabled();
    await expect(page.locator('[data-test-repository-owner]')).toBeDisabled();
    await expect(page.locator('[data-test-repository-name]')).toBeDisabled();
    await expect(page.locator('[data-test-workflow-filename]')).toBeDisabled();
    await expect(page.locator('[data-test-environment]')).toBeDisabled();
    await expect(page.locator('[data-test-add]')).toBeDisabled();

    // Resolve the deferred with an error
    deferred.resolve(HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 }));

    // Check that the error notification is displayed
    await expect(page.locator('[data-test-notification-message]')).toHaveText(
      'An error has occurred while adding the Trusted Publishing configuration: Server error',
    );

    await expect(page.locator('[data-test-publisher]')).toBeEnabled();
    await expect(page.locator('[data-test-repository-owner]')).toBeEnabled();
    await expect(page.locator('[data-test-repository-name]')).toBeEnabled();
    await expect(page.locator('[data-test-workflow-filename]')).toBeEnabled();
    await expect(page.locator('[data-test-environment]')).toBeEnabled();
    await expect(page.locator('[data-test-add]')).toBeEnabled();

    await page.click('[data-test-cancel]');
    await expect(page).toHaveURL(`/crates/${crate.name}/settings`);
    await expect(page.locator('[data-test-github-config]')).toHaveCount(0);
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
});
