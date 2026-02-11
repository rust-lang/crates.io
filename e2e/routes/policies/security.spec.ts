import { expect, test } from '@/e2e/helper';

test.describe('Route | policies.security', { tag: '@routes' }, () => {
  test('has a page title and passes a11y', async ({ page, a11y }) => {
    await page.goto('/policies/security');
    await expect(page.locator('[data-test-page-header] h1')).toHaveText('Security Information');
    await a11y.audit();
  });
});
