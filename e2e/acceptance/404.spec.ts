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
    // Pre-warm index page
    await page.goto('/');

    // Open popup to get a fresh history
    let [popup] = await Promise.all([page.waitForEvent('popup'), page.evaluate(() => window.open('/unknown-route'))]);
    await popup.waitForLoadState('domcontentloaded');

    await expect(popup.locator('[data-test-go-back]')).toBeVisible();
    await popup.click('[data-test-go-back]');
    await expect(popup).toHaveURL('/');
  });

  test('go back navigates to previous page when history exists', async ({ page }) => {
    await page.goto('/policies');
    await page.goto('/unknown-route');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await page.click('[data-test-go-back]');
    await expect(page).toHaveURL('/policies');
  });
});
