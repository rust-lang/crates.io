import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Route | team', { tag: '@routes' }, () => {
  test("shows an error message if the category can't be found", async ({ page }) => {
    await page.goto('/teams/foo');
    await expect(page).toHaveURL('/teams/foo');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Team not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('server error causes the error page to be shown', async ({ page, msw }) => {
    msw.worker.use(http.get('/api/v1/teams/:id', () => HttpResponse.json({}, { status: 500 })));

    await page.goto('/teams/foo');
    await expect(page).toHaveURL('/teams/foo');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load team data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });
});
