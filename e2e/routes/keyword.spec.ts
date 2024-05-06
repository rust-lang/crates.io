import { test, expect } from '@/e2e/helper';

test.describe('Route | keyword', { tag: '@routes' }, () => {
  test('shows an empty list if the keyword does not exist on the server', async ({ page }) => {
    await page.goto('/keywords/foo');
    await expect(page).toHaveURL('/keywords/foo');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
  });

  test('server error causes the error page to be shown', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.get('/api/v1/crates', {}, 500);
    });

    await page.goto('/keywords/foo');
    await expect(page).toHaveURL('/keywords/foo');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load crates');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });
});
