import { expect, test } from '@/e2e/helper';

test.describe('Route | settings', { tag: '@routes' }, () => {
  test('redirects to /settings/profile', async ({ page, msw }) => {
    let user = await msw.db.user.create({ login: 'johnnydee' });
    await msw.authenticateAs(user);

    await page.goto('/settings');
    await expect(page).toHaveURL('/settings/profile');
  });

  test('shows "page requires authentication" error when not logged in', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });
});
