import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | keywords', { tag: '@acceptance' }, () => {
  test('keyword/:keyword_id index default sort is recent-downloads', async ({ page, mirage, percy, a11y }) => {
    await mirage.addHook(server => {
      server.create('keyword', { keyword: 'network' });
    });

    await page.goto('/keywords/network');

    await expect(page.locator('[data-test-keyword-sort] [data-test-current-order]')).toHaveText('Recent Downloads');

    await percy.snapshot();
    await a11y.audit();
  });
});
