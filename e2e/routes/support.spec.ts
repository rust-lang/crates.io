import { test, expect } from '@/e2e/helper';

test.describe('Route | support', { tag: '@routes' }, () => {
  test('should not retain query params when exiting and then returning', async ({ page }) => {
    await page.goto('/support?inquire=crate-violation');
    await expect(page).toHaveURL('/support?inquire=crate-violation');
    let section = page.getByTestId('support-main-content').locator('section');
    await expect(section).toHaveCount(1);
    await expect(section).toHaveAttribute('data-test-id', 'crate-violation-section');

    // back to index
    await page.locator('header [href="/"]').click();
    await expect(page).toHaveURL('/');
    let link = page.locator('footer').getByRole('link', { name: 'Support', exact: true });
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

  test('LinkTo support must overwirte query', async ({ page, ember }) => {
    await ember.addHook(async owner => {
      const Service = require('@ember/service').default;
      // query params of LinkTo support's in footer will not be cleared
      class MockService extends Service {
        paramsFor() {
          return {};
        }
      }
      owner.register('service:pristine-query', MockService);
    });
    await page.goto('/support?inquire=crate-violation');
    await expect(page).toHaveURL('/support?inquire=crate-violation');
    let section = page.getByTestId('support-main-content').locator('section');
    await expect(section).toHaveCount(1);
    await expect(section).toHaveAttribute('data-test-id', 'crate-violation-section');
    // without overwriting, link in footer will contain the query params in support route
    let link = page.locator('footer').getByRole('link', { name: 'Support', exact: true });
    await expect(link).not.toHaveAttribute('href', '/support');
    await expect(link).toHaveAttribute('href', '/support?inquire=crate-violation');

    // back to index
    await page.locator('header [href="/"]').click();
    await expect(page).toHaveURL('/');
    link = page.locator('footer').getByRole('link', { name: 'Support', exact: true });
    await expect(link).toBeVisible();
    await expect(link).toHaveAttribute('href', '/support');
  });
});
