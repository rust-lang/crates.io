import { expect, test } from '@/e2e/helper';

test.describe('Route | security', { tag: '@routes' }, () => {
  test('redirects to /policies/security', async ({ page }) => {
    await page.goto('/security');
    await expect(page).toHaveURL('/policies/security');
  });
});
