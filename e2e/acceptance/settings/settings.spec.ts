import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | Settings', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ mirage }) => {
    await mirage.addHook(server => {
      let user1 = server.create('user', { name: 'blabaere' });
      let user2 = server.create('user', { name: 'thehydroimpulse' });
      let team1 = server.create('team', { org: 'org', name: 'blabaere' });
      let team2 = server.create('team', { org: 'org', name: 'thehydroimpulse' });

      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '1.0.0' });
      server.create('crate-ownership', { crate, user: user1 });
      server.create('crate-ownership', { crate, user: user2 });
      server.create('crate-ownership', { crate, team: team1 });
      server.create('crate-ownership', { crate, team: team2 });

      authenticateAs(user1);
    });
  });

  test('listing crate owners', async ({ page, percy, a11y }) => {
    await page.goto('/crates/nanomsg/settings');
    await expect(page).toHaveURL('/crates/nanomsg/settings');

    await expect(page.locator('[data-test-owners] [data-test-owner-team]')).toHaveCount(2);
    await expect(page.locator('[data-test-owners] [data-test-owner-user]')).toHaveCount(2);
    await expect(page.locator('a[href="/teams/github:org:thehydroimpulse"]').first()).toBeVisible();
    await expect(page.locator('a[href="/teams/github:org:blabaere"]').first()).toBeVisible();
    await expect(page.locator('a[href="/users/thehydroimpulse"]').first()).toBeVisible();
    await expect(page.locator('a[href="/users/blabaere"]').first()).toBeVisible();

    await percy.snapshot();
    await a11y.audit();
  });

  test('/crates/:name/owners redirects to /crates/:name/settings', async ({ page }) => {
    await page.goto('/crates/nanomsg/owners');
    await expect(page).toHaveURL('/crates/nanomsg/settings');
  });
});
