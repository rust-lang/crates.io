import { expect, test } from '@/e2e/helper';
import { loadFixtures } from '@crates-io/msw/fixtures';

test.describe('Acceptance | user page', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ page, msw }) => {
    await loadFixtures(msw.db);
    await page.goto('/users/thehydroimpulse');
  });

  test('has user display', async ({ page, percy, a11y }) => {
    await expect(page.locator('[data-test-heading] [data-test-username]')).toHaveText('thehydroimpulse');
    await expect(page.locator('[data-test-heading] [data-test-display-name]')).toHaveText('Daniel Fagnan');

    await percy.snapshot();
    await expect(page).toMatchAriaSnapshot({ name: 'aria.yml' });
    await a11y.audit();
  });

  test('has GitHub account chips in user header', async ({ page, msw }) => {
    await msw.db.user.update(q => q.where({ id: 2 }), {
      data(user) {
        user.githubAccounts = [
          { accountId: '1', login: 'github-user', avatar: null },
          { accountId: '2', login: 'thehydroimpulse', avatar: null },
        ];
      },
    });
    await page.reload();

    let accountChips = page.locator('[data-test-heading] [data-test-account-chip]');
    await expect(accountChips).toHaveCount(2);
    await expect(accountChips.locator('[data-test-handle]')).toHaveText(['github-user', 'thehydroimpulse']);
    await expect(accountChips.nth(0)).toHaveAttribute('href', 'https://github.com/github-user');
    await expect(accountChips.nth(1)).toHaveAttribute('href', 'https://github.com/thehydroimpulse');
    await expect(page.locator('[data-test-heading] [data-test-mismatch-marker]')).toHaveCount(0);
  });

  test('user details has github profile icon', async ({ page }) => {
    await expect(page.locator('[data-test-heading] [data-test-avatar]')).toHaveAttribute(
      'src',
      'https://avatars.githubusercontent.com/u/565790?v=3&s=170',
    );
  });
});
