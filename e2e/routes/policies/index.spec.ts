import { expect, test } from '@/e2e/helper';

test.describe('Route | policies', { tag: '@routes' }, () => {
  test('has a page title and passes a11y', async ({ page, a11y }) => {
    await page.goto('/policies');
    await expect(page.locator('[data-test-page-header] h1')).toHaveText('Usage Policy');
    await a11y.audit();
  });
});
