import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | Settings | Remove Owner', { tag: '@acceptance' }, () => {
  let user1, user2, team1, team2, crate;

  test.beforeEach(async ({ msw }) => {
    user1 = msw.db.user.create({ name: 'blabaere' });
    user2 = msw.db.user.create({ name: 'thehydroimpulse' });
    team1 = msw.db.team.create({ org: 'org', name: 'blabaere' });
    team2 = msw.db.team.create({ org: 'org', name: 'thehydroimpulse' });

    crate = msw.db.crate.create({ name: 'nanomsg' });
    msw.db.version.create({ crate, num: '1.0.0' });
    msw.db.crateOwnership.create({ crate, user: user1 });
    msw.db.crateOwnership.create({ crate, user: user2 });
    msw.db.crateOwnership.create({ crate, team: team1 });
    msw.db.crateOwnership.create({ crate, team: team2 });

    await msw.authenticateAs(user1);
  });

  test('remove a crate owner when owner is a user', async ({ page }) => {
    await page.goto('/crates/nanomsg/settings');
    await page.click('[data-test-owner-user="thehydroimpulse"] [data-test-remove-owner-button]');

    await expect(page.locator('[data-test-notification-message="success"]')).toHaveText(
      'User thehydroimpulse removed as crate owner',
    );
    await expect(page.locator('[data-test-owner-user]')).toHaveCount(1);
  });

  test('remove a user crate owner (error behavior)', async ({ page, msw }) => {
    // we are intentionally returning a 200 response here, because is what
    // the real backend also returns due to legacy reasons
    let error = HttpResponse.json({ errors: [{ detail: 'nope' }] });
    await msw.worker.use(http.delete('/api/v1/crates/nanomsg/owners', () => error));

    await page.goto(`/crates/${crate.name}/settings`);
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

  test('remove a team crate owner (error behavior)', async ({ page, msw }) => {
    // we are intentionally returning a 200 response here, because is what
    // the real backend also returns due to legacy reasons
    let error = HttpResponse.json({ errors: [{ detail: 'nope' }] });
    await msw.worker.use(http.delete('/api/v1/crates/nanomsg/owners', () => error));

    await page.goto(`/crates/${crate.name}/settings`);
    await page.click(`[data-test-owner-team="${team1.login}"] [data-test-remove-owner-button]`);

    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      `Failed to remove the team ${team1.org}/${team1.name} as crate owner: nope`,
    );
    await expect(page.locator('[data-test-owner-team]')).toHaveCount(2);
    await expect(page.locator('[data-test-owner-user]')).toHaveCount(2);
  });
});
