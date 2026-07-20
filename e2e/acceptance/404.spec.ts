import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | 404', { tag: '@acceptance' }, () => {
  test('/unknown-route shows a 404 page', async ({ page, percy }) => {
    await page.goto('/unknown-route');
    await expect(page).toHaveURL('/unknown-route');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('Page not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
    await percy.snapshot();
    await expect(page).toMatchAriaSnapshot({ name: 'aria.yml' });
  });

  test('go back navigates to index when there is no previous page', async ({ page }) => {
    // Can't use page.goto which adds a history entry
    await page.evaluate(() => location.replace('/unknown-route'));
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await page.click('[data-test-go-back]');
    await expect(page).toHaveURL('/');
  });

  test('go back navigates to previous page when history exists', async ({ page }) => {
    await page.goto('/policies');
    await page.goto('/unknown-route');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await page.click('[data-test-go-back]');
    await expect(page).toHaveURL('/policies');
  });
});
