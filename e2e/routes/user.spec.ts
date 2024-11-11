import { expect, test } from '@/e2e/helper';

test.describe('Route | user', { tag: '@routes' }, () => {
  test("shows an error message if the category can't be found", async ({ page }) => {
    await page.goto('/users/foo');
    await expect(page).toHaveURL('/users/foo');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: User not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('server error causes the error page to be shown', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.get('/api/v1/users/:id', {}, 500);
    });

    await page.goto('/users/foo');
    await expect(page).toHaveURL('/users/foo');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load user data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });
});
