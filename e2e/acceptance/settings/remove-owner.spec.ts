import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | Settings | Remove Owner', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ page, mirage }) => {
    await page.addInitScript(() => {
      globalThis.crate = { name: 'nanomsg' };
    });
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

      globalThis.crate = crate;
      globalThis.user2 = user2;
      globalThis.team1 = team1;
    });
  });

  test('remove a crate owner when owner is a user', async ({ page }) => {
    await page.goto('/crates/nanomsg/settings');
    await page.click('[data-test-owner-user="thehydroimpulse"] [data-test-remove-owner-button]');

    await expect(page.locator('[data-test-notification-message="success"]')).toHaveText(
      'User thehydroimpulse removed as crate owner',
    );
    await expect(page.locator('[data-test-owner-user]')).toHaveCount(1);
  });

  test('remove a user crate owner (error behavior)', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      // we are intentionally returning a 200 response here, because is what
      // the real backend also returns due to legacy reasons
      server.delete('/api/v1/crates/nanomsg/owners', { errors: [{ detail: 'nope' }] });
    });

    await page.goto('about:blank');
    let crate = await page.evaluate<{ name: string }>('crate');

    await page.goto(`/crates/${crate.name}/settings`);

    const user2 = await page.evaluate(() => JSON.parse(JSON.stringify(user2)));
    await page.click(`[data-test-owner-user="${user2.login}"] [data-test-remove-owner-button]`);

    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      `Failed to remove the user ${user2.login} as crate owner: nope`,
    );
    await expect(page.locator('[data-test-owner-user]')).toHaveCount(2);
  });

  test('remove a crate owner when owner is a team', async ({ page }) => {
    await page.goto('/crates/nanomsg/settings');
    await page.click('[data-test-owner-team="github:org:thehydroimpulse"] [data-test-remove-owner-button]');

    await expect(page.locator('[data-test-notification-message="success"]')).toHaveText(
      'Team org/thehydroimpulse removed as crate owner',
    );
    await expect(page.locator('[data-test-owner-team]')).toHaveCount(1);
  });

  test('remove a team crate owner (error behavior)', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      // we are intentionally returning a 200 response here, because is what
      // the real backend also returns due to legacy reasons
      server.delete('/api/v1/crates/nanomsg/owners', { errors: [{ detail: 'nope' }] });
    });

    await page.goto('about:blank');
    let crate = await page.evaluate<{ name: string }>('crate');

    await page.goto(`/crates/${crate.name}/settings`);

    let team1 = await page.evaluate(() => JSON.parse(JSON.stringify(team1)));
    await page.click(`[data-test-owner-team="${team1.login}"] [data-test-remove-owner-button]`);

    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      `Failed to remove the team ${team1.org}/${team1.name} as crate owner: nope`,
    );
    await expect(page.locator('[data-test-owner-team]')).toHaveCount(2);
    await expect(page.locator('[data-test-owner-user]')).toHaveCount(2);
  });
});
