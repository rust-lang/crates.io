import { expect, test } from '@/e2e/helper';
import { loadFixtures } from '@crates-io/msw/fixtures';

test.describe('Acceptance | org page', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ page, msw }) => {
    await loadFixtures(msw.db);
    await page.goto('/orgs/github:org');
  });

  test('has team organization display', async ({ page, percy, a11y }) => {
    await expect(page.locator('[data-test-heading] [data-test-org-name]')).toHaveText('github:org');
    await expect(page.locator('[data-test-heading] [data-test-org-teams]')).toHaveText('teams:');
    await expect(page.locator('[data-test-heading] [data-test-org-teams-list] li')).toHaveCount(2);
    await expect(page.locator('[data-test-heading] [data-test-org-teams-list] li[data-test-row="0"]')).toHaveText(
      'thehydroimpulseteam',
    );
    await expect(page.locator('[data-test-heading] [data-test-org-teams-list] li[data-test-row="1"]')).toHaveText(
      'Team Benoît Labaere',
    );

    await percy.snapshot();
    await expect(page).toMatchAriaSnapshot({ name: 'aria.yml' });
    await a11y.audit();
  });

  test('has link to github in org header', async ({ page }) => {
    await expect(page.locator('[data-test-heading] [data-test-github-link]')).toHaveAttribute(
      'href',
      'https://github.com/org_test',
    );
  });

  test('organization details has github profile icon', async ({ page }) => {
    await expect(page.locator('[data-test-heading] [data-test-avatar] img')).toHaveAttribute(
      'src',
      'https://avatars.githubusercontent.com/u/565790?v=3&s=256',
    );
  });
});
