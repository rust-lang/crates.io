import { expect, test } from '@/e2e/helper';
import { loadFixtures } from '@crates-io/msw/fixtures';

test.describe('Acceptance | user page', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ page, msw }) => {
    await loadFixtures(msw.db);
    await page.goto('/users/thehydroimpulse');
  });

  test('has user display', async ({ page, percy, a11y }) => {
    await expect(page.locator('[data-test-heading] [data-test-username]')).toHaveText('thehydroimpulse');

    await percy.snapshot();
    await expect(page).toMatchAriaSnapshot({ name: 'aria.yml' });
    await a11y.audit();
  });

  test('has GitHub account chip in user header', async ({ page }) => {
    let accountChip = page.locator('[data-test-heading] [data-test-account-chip]');

    await expect(accountChip).toHaveAttribute('href', 'https://github.com/thehydroimpulse');
    await expect(accountChip.locator('[data-test-handle]')).toHaveText('thehydroimpulse');
  });

  test('user details has github profile icon', async ({ page }) => {
    await expect(page.locator('[data-test-heading] [data-test-avatar]')).toHaveAttribute(
      'src',
      'https://avatars.githubusercontent.com/u/565790?v=3&s=170',
    );
  });
});
