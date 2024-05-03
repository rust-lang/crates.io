import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | team page', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/teams/github:org:thehydroimpulse');
  });

  test('has team organization display', async ({ page, percy, a11y }) => {
    await expect(page.locator('[data-test-heading] [data-test-org-name]')).toHaveText('org');
    await expect(page.locator('[data-test-heading] [data-test-team-name]')).toHaveText('thehydroimpulseteam');

    await percy.snapshot();
    await a11y.audit();
  });

  test('has link to github in team header', async ({ page }) => {
    await expect(page.locator('[data-test-heading] [data-test-github-link]')).toHaveAttribute(
      'href',
      'https://github.com/org_test',
    );
  });

  test('team organization details has github profile icon', async ({ page }) => {
    await expect(page.locator('[data-test-heading] [data-test-avatar]')).toHaveAttribute(
      'src',
      'https://avatars.githubusercontent.com/u/565790?v=3&s=170',
    );
  });
});
