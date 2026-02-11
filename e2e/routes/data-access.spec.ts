import { expect, test } from '@/e2e/helper';

test.describe('Route | data-access', { tag: '@routes' }, () => {
  test('has a page title and passes a11y', async ({ page, a11y }) => {
    await page.goto('/data-access');
    await expect(page.locator('[data-test-page-header] h1')).toHaveText('Data Access Policy');
    await a11y.audit();
  });
});
