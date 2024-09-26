import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | support page', { tag: '@acceptance' }, () => {
  test('shows an inquire list', async ({ page, percy, a11y }) => {
    await page.goto('/support');
    await expect(page).toHaveURL('/support');

    await expect(page.getByTestId('support-main-content').locator('section')).toHaveCount(1);
    await expect(page.getByTestId('inquire-list-section')).toBeVisible();
    const inquireList = page.getByTestId('inquire-list');
    await expect(inquireList).toBeVisible();
    await expect(inquireList.locator(page.getByRole('listitem'))).toHaveText(['Report a crate that violates policies']);

    await percy.snapshot();
    await a11y.audit();
  });

  test('shows an inquire list if given inquire is not supported', async ({ page }) => {
    await page.goto('/support?inquire=not-supported-inquire');
    await expect(page).toHaveURL('/support?inquire=not-supported-inquire');

    await expect(page.getByTestId('support-main-content').locator('section')).toHaveCount(1);
    await expect(page.getByTestId('inquire-list-section')).toBeVisible();
    const inquireList = page.getByTestId('inquire-list');
    await expect(inquireList).toBeVisible();
    await expect(inquireList.locator(page.getByRole('listitem'))).toHaveText(['Report a crate that violates policies']);
  });
});
