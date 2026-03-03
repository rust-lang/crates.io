import { expect, test } from '@/e2e/helper';

test.describe('Route | rate-limits', { tag: '@routes' }, () => {
  test('has a page title and passes a11y', async ({ page, a11y }) => {
    await page.goto('/docs/rate-limits');
    await expect(page.locator('[data-test-page-header] h1')).toHaveText('Publishing Rate Limits');
    await a11y.audit();
  });
});
