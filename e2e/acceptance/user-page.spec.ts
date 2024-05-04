import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | user page', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });
    await page.goto('/users/thehydroimpulse');
  });

  test('has user display', async ({ page, percy, a11y }) => {
    await expect(page.locator('[data-test-heading] [data-test-username]')).toHaveText('thehydroimpulse');

    await percy.snapshot();
    await a11y.audit();
  });

  test('has link to github in user header', async ({ page }) => {
    await expect(page.locator('[data-test-heading] [data-test-user-link]')).toHaveAttribute(
      'href',
      'https://github.com/thehydroimpulse',
    );
  });

  test('user details has github profile icon', async ({ page }) => {
    await expect(page.locator('[data-test-heading] [data-test-avatar]')).toHaveAttribute(
      'src',
      'https://avatars.githubusercontent.com/u/565790?v=3&s=170',
    );
  });
});
