import { expect, test } from '@/e2e/helper';
import { defer } from '@/e2e/deferred';
import { http, HttpResponse } from 'msw';

test.describe('Route | crate.settings', { tag: '@routes' }, () => {
  async function prepare(msw) {
    let user = msw.db.user.create();

    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate });
    msw.db.crateOwnership.create({ crate, user });

    await msw.authenticateAs(user);

    return { crate, user };
  }

  test('unauthenticated', async ({ msw, page }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate });

    await page.goto('/crates/foo/settings');
    await expect(page).toHaveURL('/crates/foo/settings');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });

  test('not an owner', async ({ msw, page }) => {
    let user1 = msw.db.user.create();
    await msw.authenticateAs(user1);

    let user2 = msw.db.user.create();
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate });
    msw.db.crateOwnership.create({ crate, user: user2 });

    await page.goto('/crates/foo/settings');
    await expect(page).toHaveURL('/crates/foo/settings');
    await expect(page.locator('[data-test-title]')).toHaveText('This page is only accessible by crate owners');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
  });

  test('happy path', async ({ msw, page }) => {
    let { user } = await prepare(msw);

    await page.goto('/crates/foo/settings');
    await expect(page).toHaveURL('/crates/foo/settings');

    await expect(page.locator('[data-test-owners]')).toBeVisible();
    await expect(page.locator('[data-test-add-owner-button]')).toBeVisible();
    await expect(page.locator(`[data-test-owner-user="${user.login}"]`)).toBeVisible();
    await expect(page.locator('[data-test-remove-owner-button]')).toBeVisible();

    await expect(page.locator('[data-test-trusted-publishing]')).toBeVisible();
    await expect(page.locator('[data-test-no-config]')).toBeVisible();
    await expect(page.locator('[data-test-github-config]')).not.toBeVisible();

    await expect(page.locator('[data-test-delete-button]')).toBeVisible();
  });

  test.describe('Trusted Publishing', () => {
    test('mixed GitHub and GitLab configs', async ({ msw, page, percy }) => {
      const { crate } = await prepare(msw);

      // Create GitHub config
      msw.db.trustpubGithubConfig.create({
        crate,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      });

      // Create GitLab config
      msw.db.trustpubGitlabConfig.create({
        crate,
        namespace: 'johndoe',
        namespace_id: '1234',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
        environment: 'production',
      });

      await page.goto('/crates/foo/settings');
      await expect(page).toHaveURL('/crates/foo/settings');

      await percy.snapshot();

      // Check that both GitHub and GitLab configs are displayed
      await expect(page.locator('[data-test-trusted-publishing]')).toBeVisible();
      await expect(page.locator('[data-test-add-trusted-publisher-button]')).toBeVisible();
      await expect(page.locator('[data-test-github-config]')).toHaveCount(1);
      await expect(page.locator('[data-test-gitlab-config]')).toHaveCount(1);

      // Verify GitHub config
      await expect(page.locator('[data-test-github-config="1"] td:nth-child(1)')).toHaveText('GitHub');
      let details = page.locator('[data-test-github-config="1"] td:nth-child(2)');
      await expect(details).toContainText('Repository: rust-lang/crates.io');
      await expect(details).toContainText('Workflow: ci.yml');

      // Verify GitLab config
      await expect(page.locator('[data-test-gitlab-config="1"] td:nth-child(1)')).toHaveText('GitLab');
      details = page.locator('[data-test-gitlab-config="1"] td:nth-child(2)');
      await expect(details).toContainText('Repository: johndoe/crates.io');
      await expect(details).toContainText('Namespace ID: 1234');
      await expect(details).toContainText('Workflow: .gitlab-ci.yml');
      await expect(details).toContainText('Environment: production');

      await expect(page.locator('[data-test-no-config]')).not.toBeVisible();
    });

    test.describe('GitHub', () => {
      test('happy path', async ({ msw, page, percy }) => {
        const { crate } = await prepare(msw);

        // Create two GitHub configs for the crate
        msw.db.trustpubGithubConfig.create({
          crate,
          repository_owner: 'rust-lang',
          repository_name: 'crates.io',
          workflow_filename: 'ci.yml',
        });

        msw.db.trustpubGithubConfig.create({
          crate,
          repository_owner: 'johndoe',
          repository_name: 'crates.io',
          workflow_filename: 'release.yml',
          environment: 'release',
        });

        await page.goto('/crates/foo/settings');
        await expect(page).toHaveURL('/crates/foo/settings');

        await expect(page.locator('[data-test-trusted-publishing]')).toBeVisible();
        await expect(page.locator('[data-test-add-trusted-publisher-button]')).toBeVisible();
        await expect(page.locator('[data-test-github-config]')).toHaveCount(2);
        await expect(page.locator('[data-test-github-config="1"] td:nth-child(1)')).toHaveText('GitHub');
        let details = page.locator('[data-test-github-config="1"] td:nth-child(2)');
        await expect(details).toContainText('Repository: rust-lang/crates.io');
        await expect(details).toContainText('Workflow: ci.yml');
        await expect(details).not.toContainText('Environment');
        await expect(page.locator('[data-test-github-config="1"] [data-test-remove-config-button]')).toBeVisible();
        await expect(page.locator('[data-test-github-config="2"] td:nth-child(1)')).toHaveText('GitHub');
        details = page.locator('[data-test-github-config="2"] td:nth-child(2)');
        await expect(details).toContainText('Repository: johndoe/crates.io');
        await expect(details).toContainText('Workflow: release.yml');
        await expect(details).toContainText('Environment: release');
        await expect(page.locator('[data-test-github-config="2"] [data-test-remove-config-button]')).toBeVisible();
        await expect(page.locator('[data-test-no-config]')).not.toBeVisible();

        // Click the remove button
        await page.click('[data-test-github-config="2"] [data-test-remove-config-button]');

        // Check that the config is no longer displayed
        await expect(page.locator('[data-test-github-config]')).toHaveCount(1);
        details = page.locator('[data-test-github-config="1"] td:nth-child(2)');
        await expect(details).toContainText('Repository: rust-lang/crates.io');
        await expect(page.locator('[data-test-notification-message]')).toHaveText(
          'Trusted Publishing configuration removed successfully',
        );
      });

      test('deletion failure', async ({ msw, page, percy }) => {
        let { crate } = await prepare(msw);

        // Create a GitHub config for the crate
        let config = msw.db.trustpubGithubConfig.create({
          crate,
          repository_owner: 'rust-lang',
          repository_name: 'crates.io',
          workflow_filename: 'ci.yml',
          environment: 'release',
        });

        // Mock the server to return an error when trying to delete the config
        await msw.worker.use(
          http.delete(`/api/v1/trusted_publishing/github_configs/${config.id}`, () => {
            return HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 });
          }),
        );

        await page.goto(`/crates/${crate.name}/settings`);
        await expect(page).toHaveURL(`/crates/${crate.name}/settings`);
        await expect(page.locator('[data-test-github-config]')).toHaveCount(1);

        await page.click('[data-test-remove-config-button]');
        await expect(page.locator('[data-test-github-config]')).toHaveCount(1);
        await expect(page.locator('[data-test-notification-message]')).toHaveText(
          'Failed to remove Trusted Publishing configuration: Server error',
        );
      });
    });

    test.describe('GitLab', () => {
      test('happy path', async ({ msw, page, percy }) => {
        const { crate } = await prepare(msw);

        // Create two GitLab configs for the crate
        msw.db.trustpubGitlabConfig.create({
          crate,
          namespace: 'rust-lang',
          project: 'crates.io',
          workflow_filepath: '.gitlab-ci.yml',
        });

        msw.db.trustpubGitlabConfig.create({
          crate,
          namespace: 'johndoe',
          namespace_id: '1234',
          project: 'crates.io',
          workflow_filepath: '.gitlab-ci.yml',
          environment: 'release',
        });

        await page.goto('/crates/foo/settings');
        await expect(page).toHaveURL('/crates/foo/settings');

        await expect(page.locator('[data-test-trusted-publishing]')).toBeVisible();
        await expect(page.locator('[data-test-add-trusted-publisher-button]')).toBeVisible();
        await expect(page.locator('[data-test-gitlab-config]')).toHaveCount(2);
        await expect(page.locator('[data-test-gitlab-config="1"] td:nth-child(1)')).toHaveText('GitLab');
        let details = page.locator('[data-test-gitlab-config="1"] td:nth-child(2)');
        await expect(details).toContainText('Repository: rust-lang/crates.io');
        await expect(details).toContainText('Namespace ID: (not yet set)');
        await expect(details).toContainText('Workflow: .gitlab-ci.yml');
        await expect(details).not.toContainText('Environment');
        await expect(page.locator('[data-test-gitlab-config="1"] [data-test-remove-config-button]')).toBeVisible();
        await expect(page.locator('[data-test-gitlab-config="2"] td:nth-child(1)')).toHaveText('GitLab');
        details = page.locator('[data-test-gitlab-config="2"] td:nth-child(2)');
        await expect(details).toContainText('Repository: johndoe/crates.io');
        await expect(details).toContainText('Namespace ID: 1234');
        await expect(details).toContainText('Workflow: .gitlab-ci.yml');
        await expect(details).toContainText('Environment: release');
        await expect(page.locator('[data-test-gitlab-config="2"] [data-test-remove-config-button]')).toBeVisible();
        await expect(page.locator('[data-test-no-config]')).not.toBeVisible();

        // Click the remove button
        await page.click('[data-test-gitlab-config="2"] [data-test-remove-config-button]');

        // Check that the config is no longer displayed
        await expect(page.locator('[data-test-gitlab-config]')).toHaveCount(1);
        details = page.locator('[data-test-gitlab-config="1"] td:nth-child(2)');
        await expect(details).toContainText('Repository: rust-lang/crates.io');
        await expect(page.locator('[data-test-notification-message]')).toHaveText(
          'Trusted Publishing configuration removed successfully',
        );
      });

      test('deletion failure', async ({ msw, page, percy }) => {
        let { crate } = await prepare(msw);

        // Create a GitLab config for the crate
        let config = msw.db.trustpubGitlabConfig.create({
          crate,
          namespace: 'rust-lang',
          namespace_id: '1234',
          project: 'crates.io',
          workflow_filepath: '.gitlab-ci.yml',
          environment: 'release',
        });

        // Mock the server to return an error when trying to delete the config
        await msw.worker.use(
          http.delete(`/api/v1/trusted_publishing/gitlab_configs/${config.id}`, () => {
            return HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 });
          }),
        );

        await page.goto(`/crates/${crate.name}/settings`);
        await expect(page).toHaveURL(`/crates/${crate.name}/settings`);
        await expect(page.locator('[data-test-gitlab-config]')).toHaveCount(1);

        await page.click('[data-test-remove-config-button]');
        await expect(page.locator('[data-test-gitlab-config]')).toHaveCount(1);
        await expect(page.locator('[data-test-notification-message]')).toHaveText(
          'Failed to remove Trusted Publishing configuration: Server error',
        );
      });
    });

    test.describe('trustpub_only warning banner', () => {
      test('hidden when flag is false', async ({ msw, page }) => {
        await prepare(msw);

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-warning]')).not.toBeVisible();
      });

      test('hidden when flag is true and configs exist', async ({ msw, page }) => {
        const { crate } = await prepare(msw);
        msw.db.crate.update({ where: { id: { equals: crate.id } }, data: { trustpubOnly: true } });

        msw.db.trustpubGithubConfig.create({
          crate,
          repository_owner: 'rust-lang',
          repository_name: 'crates.io',
          workflow_filename: 'ci.yml',
        });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-warning]')).not.toBeVisible();
      });

      test('shown when flag is true but no configs exist', async ({ msw, page, percy }) => {
        const { crate } = await prepare(msw);
        msw.db.crate.update({ where: { id: { equals: crate.id } }, data: { trustpubOnly: true } });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-warning]')).toBeVisible();
        await expect(page.locator('[data-test-trustpub-only-warning]')).toHaveText(
          'Trusted publishing is required but no publishers are configured. Publishing to this crate is currently blocked.',
        );

        await percy.snapshot();
      });

      test('disappears when checkbox is unchecked', async ({ msw, page }) => {
        const { crate } = await prepare(msw);
        msw.db.crate.update({ where: { id: { equals: crate.id } }, data: { trustpubOnly: true } });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-warning]')).toBeVisible();

        await page.click('[data-test-trustpub-only-checkbox] [data-test-checkbox]');

        await expect(page.locator('[data-test-trustpub-only-warning]')).not.toBeVisible();
      });

      test('appears when last config is removed', async ({ msw, page }) => {
        const { crate } = await prepare(msw);
        msw.db.crate.update({ where: { id: { equals: crate.id } }, data: { trustpubOnly: true } });

        msw.db.trustpubGithubConfig.create({
          crate,
          repository_owner: 'rust-lang',
          repository_name: 'crates.io',
          workflow_filename: 'ci.yml',
        });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-warning]')).not.toBeVisible();

        await page.click('[data-test-github-config="1"] [data-test-remove-config-button]');

        await expect(page.locator('[data-test-trustpub-only-warning]')).toBeVisible();
      });
    });

    test.describe('trustpub_only checkbox', () => {
      test('hidden when no configs and flag is false', async ({ msw, page }) => {
        await prepare(msw);

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-checkbox]')).not.toBeVisible();
      });

      test('visible when GitHub configs exist', async ({ msw, page }) => {
        const { crate } = await prepare(msw);

        msw.db.trustpubGithubConfig.create({
          crate,
          repository_owner: 'rust-lang',
          repository_name: 'crates.io',
          workflow_filename: 'ci.yml',
        });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-checkbox]')).toBeVisible();
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).not.toBeChecked();
      });

      test('visible when GitLab configs exist', async ({ msw, page }) => {
        const { crate } = await prepare(msw);

        msw.db.trustpubGitlabConfig.create({
          crate,
          namespace: 'rust-lang',
          project: 'crates.io',
          workflow_filepath: '.gitlab-ci.yml',
        });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-checkbox]')).toBeVisible();
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).not.toBeChecked();
      });

      test('visible when flag is true but no configs', async ({ msw, page }) => {
        const { crate } = await prepare(msw);
        msw.db.crate.update({ where: { id: { equals: crate.id } }, data: { trustpubOnly: true } });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-checkbox]')).toBeVisible();
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).toBeChecked();
      });

      test('stays visible after disabling when no configs exist', async ({ msw, page }) => {
        const { crate } = await prepare(msw);
        msw.db.crate.update({ where: { id: { equals: crate.id } }, data: { trustpubOnly: true } });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-checkbox]')).toBeVisible();
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).toBeChecked();

        await page.click('[data-test-trustpub-only-checkbox] [data-test-checkbox]');

        // Checkbox stays visible after disabling (within same page visit)
        await expect(page.locator('[data-test-trustpub-only-checkbox]')).toBeVisible();
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).not.toBeChecked();

        // After navigating away and back, checkbox should be hidden
        await page.goto('/crates/foo');
        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-checkbox]')).not.toBeVisible();
      });

      test('enabling trustpub_only', async ({ msw, page }) => {
        const { crate } = await prepare(msw);

        msw.db.trustpubGithubConfig.create({
          crate,
          repository_owner: 'rust-lang',
          repository_name: 'crates.io',
          workflow_filename: 'ci.yml',
        });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).not.toBeChecked();
        expect(msw.db.crate.findFirst({ where: { name: { equals: crate.name } } })?.trustpubOnly).toBe(false);

        await page.click('[data-test-trustpub-only-checkbox] [data-test-checkbox]');

        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).toBeChecked();
        expect(msw.db.crate.findFirst({ where: { name: { equals: crate.name } } })?.trustpubOnly).toBe(true);
      });

      test('disabling trustpub_only', async ({ msw, page }) => {
        const { crate } = await prepare(msw);
        msw.db.crate.update({ where: { id: { equals: crate.id } }, data: { trustpubOnly: true } });

        msw.db.trustpubGithubConfig.create({
          crate,
          repository_owner: 'rust-lang',
          repository_name: 'crates.io',
          workflow_filename: 'ci.yml',
        });

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).toBeChecked();
        expect(msw.db.crate.findFirst({ where: { name: { equals: crate.name } } })?.trustpubOnly).toBe(true);

        await page.click('[data-test-trustpub-only-checkbox] [data-test-checkbox]');

        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).not.toBeChecked();
        expect(msw.db.crate.findFirst({ where: { name: { equals: crate.name } } })?.trustpubOnly).toBe(false);
      });

      test('loading and error state', async ({ msw, page }) => {
        const { crate } = await prepare(msw);

        msw.db.trustpubGithubConfig.create({
          crate,
          repository_owner: 'rust-lang',
          repository_name: 'crates.io',
          workflow_filename: 'ci.yml',
        });

        let deferred = defer();
        msw.worker.use(http.patch('/api/v1/crates/:name', () => deferred.promise));

        await page.goto('/crates/foo/settings');

        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).toBeVisible();
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-spinner]')).not.toBeVisible();

        await page.click('[data-test-trustpub-only-checkbox] [data-test-checkbox]');

        // Check loading state
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-spinner]')).toBeVisible();
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).not.toBeVisible();

        // Resolve with error
        deferred.resolve(HttpResponse.json({ errors: [{ detail: 'Server error' }] }, { status: 500 }));

        // Check that spinner is gone and checkbox is back
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-checkbox]')).toBeVisible();
        await expect(page.locator('[data-test-trustpub-only-checkbox] [data-test-spinner]')).not.toBeVisible();
        await expect(page.locator('[data-test-notification-message]')).toHaveText('Server error');
      });
    });
  });
});
