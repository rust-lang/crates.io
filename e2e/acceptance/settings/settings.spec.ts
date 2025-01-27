import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | Settings', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ msw }) => {
    let user1 = msw.db.user.create({ name: 'blabaere' });
    let user2 = msw.db.user.create({ name: 'thehydroimpulse' });
    let team1 = msw.db.team.create({ org: 'org', name: 'blabaere' });
    let team2 = msw.db.team.create({ org: 'org', name: 'thehydroimpulse' });

    let crate = msw.db.crate.create({ name: 'nanomsg' });
    msw.db.version.create({ crate, num: '1.0.0' });
    msw.db.crateOwnership.create({ crate, user: user1 });
    msw.db.crateOwnership.create({ crate, user: user2 });
    msw.db.crateOwnership.create({ crate, team: team1 });
    msw.db.crateOwnership.create({ crate, team: team2 });

    await msw.authenticateAs(user1);
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
