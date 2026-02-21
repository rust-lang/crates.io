import { expect, test } from '@/e2e/helper';

test.describe('Route | support', { tag: '@routes' }, () => {
  test('footer should always point to /support without query parameters', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('footer [data-test-support-link]')).toHaveAttribute('href', '/support');

    await page.goto('/support?inquire=crate-violation&crate=foo');
    await expect(page.locator('footer [data-test-support-link]')).toHaveAttribute('href', '/support');

    await page.locator('header [href="/"]').click();
    await expect(page.locator('footer [data-test-support-link]')).toHaveAttribute('href', '/support');
  });

  test('should not retain query params when exiting and then returning', async ({ page, msw }) => {
    let user = await msw.db.user.create({});
    await msw.authenticateAs(user);

    await page.goto('/support?inquire=crate-violation');
    await expect(page).toHaveURL('/support?inquire=crate-violation');
    let section = page.getByTestId('support-main-content').locator('section');
    await expect(section).toHaveCount(1);
    await expect(section).toHaveAttribute('data-test-id', 'crate-violation-section');

    // back to index
    await page.locator('header [href="/"]').click();
    await expect(page).toHaveURL('/');
    let link = page.locator('footer [data-test-support-link]');
    await expect(link).toBeVisible();
    await expect(link).toHaveAttribute('href', '/support');

    // goto support
    await link.click();
    await expect(page).toHaveURL('/support');
    section = page.getByTestId('support-main-content').locator('section');
    await expect(section).toHaveCount(1);
    await expect(section).toHaveAttribute('data-test-id', 'inquire-list-section');
    await page.getByTestId('link-crate-violation').click();
    await expect(page).toHaveURL('/support?inquire=crate-violation');
  });
});
