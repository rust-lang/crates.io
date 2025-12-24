import { expect, test } from '@/e2e/helper';

test.describe('Route | me/crates', { tag: '@routes' }, () => {
  test('redirects to user profile page', async ({ page, msw }) => {
    let user = await msw.db.user.create({ login: 'johnnydee' });
    await msw.authenticateAs(user);

    await page.goto('/me/crates?page=2&sort=downloads');
    await expect(page).toHaveURL('/users/johnnydee?page=2&sort=downloads');
  });
});
