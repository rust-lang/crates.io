import { expect, test } from '@/e2e/helper';

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

    // Disabled for now, until we make this feature publicly available
    // await expect(page.locator('[data-test-trusted-publishing]')).toBeVisible();
    // await expect(page.locator('[data-test-no-config]')).toBeVisible();
    // await expect(page.locator('[data-test-github-config]')).not.toBeVisible();
    await expect(page.locator('[data-test-trusted-publishing]')).not.toBeVisible();

    await expect(page.locator('[data-test-delete-button]')).toBeVisible();
  });

  test.describe('Trusted Publishing', () => {
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

      await percy.snapshot();

      await expect(page.locator('[data-test-trusted-publishing]')).toBeVisible();
      await expect(page.locator('[data-test-github-config]')).toHaveCount(2);
      await expect(page.locator('[data-test-github-config="1"] td:nth-child(1)')).toHaveText('GitHub');
      let details = page.locator('[data-test-github-config="1"] td:nth-child(2)');
      await expect(details).toContainText('Repository: rust-lang/crates.io');
      await expect(details).toContainText('Workflow: ci.yml');
      await expect(details).not.toContainText('Environment');
      await expect(page.locator('[data-test-github-config="2"] td:nth-child(1)')).toHaveText('GitHub');
      details = page.locator('[data-test-github-config="2"] td:nth-child(2)');
      await expect(details).toContainText('Repository: johndoe/crates.io');
      await expect(details).toContainText('Workflow: release.yml');
      await expect(details).toContainText('Environment: release');
      await expect(page.locator('[data-test-no-config]')).not.toBeVisible();
    });
  });
});
