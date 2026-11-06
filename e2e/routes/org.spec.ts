import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Route | org', { tag: '@routes' }, () => {
  test('server error causes the error page to be shown', async ({ page, msw }) => {
    msw.worker.use(http.get('/api/v1/orgs/:id', () => HttpResponse.json({}, { status: 500 })));

    await page.goto('/orgs/github:foo');
    await expect(page).toHaveURL('/orgs/github:foo');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('github:foo: Failed to load org data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('empty teams listing', async ({ page, msw }) => {
    msw.worker.use(http.get('/api/v1/orgs/:id', () => HttpResponse.json({ teams: [] }, { status: 200 })));

    await page.goto('/orgs/github:foo');
    await expect(page).toHaveURL('/orgs/github:foo');
    await expect(page.locator('[data-test-org-name]')).toHaveText('github:foo');
    await expect(page.locator('[data-test-org-teams]')).toHaveText('no teams found');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });
});
