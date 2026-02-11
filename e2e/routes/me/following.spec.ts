import { expect, test } from '@/e2e/helper';

test.describe('Route | me/following', { tag: '@routes' }, () => {
  test('shows "page requires authentication" error when not logged in', async ({ page }) => {
    await page.goto('/me/following');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });

  test('shows empty list for user with no followed crates', async ({ page, msw }) => {
    let user = await msw.db.user.create({ followedCrates: [] });
    await msw.authenticateAs(user);

    await page.goto('/me/following');
    await expect(page.locator('[data-test-page-header] h1')).toHaveText('Followed Crates');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
    await expect(page.locator('[data-test-total-rows]')).toHaveText('0');
  });

  test('shows followed crates with pagination', async ({ page, msw }) => {
    let followedCrates = [];
    for (let i = 0; i < 12; i++) {
      let crate = await msw.db.crate.create({});
      await msw.db.version.create({ crate });
      followedCrates.push(crate);
    }

    // create some crates that are NOT followed
    for (let i = 0; i < 3; i++) {
      let crate = await msw.db.crate.create({});
      await msw.db.version.create({ crate });
    }

    let user = await msw.db.user.create({ followedCrates });
    await msw.authenticateAs(user);

    await page.goto('/me/following');
    await expect(page.locator('[data-test-page-header] h1')).toHaveText('Followed Crates');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(10);
    await expect(page.locator('[data-test-current-rows]')).toHaveText('1-10');
    await expect(page.locator('[data-test-total-rows]')).toHaveText('12');

    await page.click('[data-test-pagination-next]');
    await expect(page).toHaveURL('/me/following?page=2');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(2);
    await expect(page.locator('[data-test-current-rows]')).toHaveText('11-12');
    await expect(page.locator('[data-test-total-rows]')).toHaveText('12');
  });
});
