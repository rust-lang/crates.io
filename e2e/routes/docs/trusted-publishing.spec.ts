import { expect, test } from '@/e2e/helper';

test.describe('Route | docs.trusted-publishing', { tag: '@routes' }, () => {
  test('has a page title and passes a11y', async ({ page, a11y }) => {
    await page.goto('/docs/trusted-publishing');
    await expect(page.locator('[data-test-page-header] h1')).toHaveText('Trusted Publishing');
    await a11y.audit();
  });
});
