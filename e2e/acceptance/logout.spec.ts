import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | Logout', { tag: '@acceptance' }, () => {
  test('successful logout', async ({ page, msw }) => {
    let user = await msw.db.user.create({ name: 'John Doe' });
    await msw.authenticateAs(user);

    await page.goto('/crates');
    await expect(page).toHaveURL('/crates');

    let toggle = page.locator('[data-test-user-menu-toggle]');
    await expect(toggle).toHaveText('John Doe');

    await toggle.click();
    await page.locator('[data-test-user-menu] [data-test-logout-button]').click();

    await page.waitForURL('/');
  });
});
