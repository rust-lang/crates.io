import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Route | category', { tag: '@routes' }, () => {
  test("shows an error message if the category can't be found", async ({ page }) => {
    await page.goto('/categories/foo');
    await expect(page).toHaveURL('/categories/foo');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Category not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('server error causes the error page to be shown', async ({ page, msw }) => {
    msw.worker.use(http.get('/api/v1/categories/:categoryId', () => HttpResponse.json({}, { status: 500 })));

    await page.goto('/categories/foo');
    await expect(page).toHaveURL('/categories/foo');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load category data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('updates the search field when the categories route is accessed', async ({ page, msw }) => {
    msw.db.category.create({ category: 'foo' });

    const searchInput = page.locator('[data-test-search-input]');
    await page.goto('/');
    await page.waitForURL('/');
    await expect(searchInput).toHaveValue('');

    // favor navigation via link click over page.goto
    await page.getByRole('link', { name: 'foo 0 crates' }).click();
    await page.waitForURL('/categories/foo');
    await expect(searchInput).toHaveValue('category:foo ');

    // favor navigation via link click over page.goto
    await page.getByRole('link', { name: 'crates.io', exact: true }).click();
    await page.waitForURL('/');
    await expect(searchInput).toHaveValue('');
  });
});
