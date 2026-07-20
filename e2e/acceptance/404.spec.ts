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
    await page.goto('/unknown-route');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await page.click('[data-test-go-back]');
    // Svelte doesn't update URL during tests for some reason, so the following assertion only works in actual browsers
    // await expect(page).toHaveURL('/');
    // Instead, assert we are no longer on 404 page...
    await expect(page.locator('[data-test-404-page]')).toBeHidden();
    // ...but on index page instead
    await expect(page).toHaveTitle('crates.io: Rust Package Registry');
  });

  test('go back navigates to previous page when history exists', async ({ page }) => {
    await page.goto('/policies');
    await page.goto('/unknown-route');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await page.click('[data-test-go-back]');
    await expect(page).toHaveURL('/policies');
  });
});
