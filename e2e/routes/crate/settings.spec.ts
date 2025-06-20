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
    await expect(page.locator('[data-test-delete-button]')).toBeVisible();
  });
});
