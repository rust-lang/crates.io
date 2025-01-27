import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | keywords', { tag: '@acceptance' }, () => {
  test('keyword/:keyword_id index default sort is recent-downloads', async ({ page, msw, percy, a11y }) => {
    msw.db.keyword.create({ keyword: 'network' });

    await page.goto('/keywords/network');

    await expect(page.locator('[data-test-keyword-sort] [data-test-current-order]')).toHaveText('Recent Downloads');

    await percy.snapshot();
    await a11y.audit();
  });
});
