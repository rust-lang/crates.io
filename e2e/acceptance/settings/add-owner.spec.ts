import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | Settings | Add Owner', { tag: '@acceptance' }, () => {
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

  test('attempting to add owner without username', async ({ page }) => {
    await page.goto('/crates/nanomsg/settings');
    await page.fill('input[name="username"]', '');
    await expect(page.locator('[data-test-save-button]')).toBeDisabled();
  });

  test('attempting to add non-existent owner', async ({ page }) => {
    await page.goto('/crates/nanomsg/settings');
    await page.fill('input[name="username"]', 'spookyghostboo');
    await page.click('[data-test-save-button]');

    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Error sending invite: could not find user with login `spookyghostboo`',
    );
    await expect(page.locator('[data-test-owners] [data-test-owner-team]')).toHaveCount(2);
    await expect(page.locator('[data-test-owners] [data-test-owner-user]')).toHaveCount(2);
  });

  test('add a new owner', async ({ page, msw }) => {
    msw.db.user.create({ name: 'iain8' });

    await page.goto('/crates/nanomsg/settings');
    await page.fill('input[name="username"]', 'iain8');
    await page.click('[data-test-save-button]');

    await expect(page.locator('[data-test-notification-message="success"]')).toHaveText(
      'An invite has been sent to iain8',
    );
    await expect(page.locator('[data-test-owners] [data-test-owner-team]')).toHaveCount(2);
    await expect(page.locator('[data-test-owners] [data-test-owner-user]')).toHaveCount(2);
  });

  test('add a team owner', async ({ page, msw }) => {
    msw.db.user.create({ name: 'iain8' });
    msw.db.team.create({ org: 'rust-lang', name: 'crates-io' });

    await page.goto('/crates/nanomsg/settings');
    await page.fill('input[name="username"]', 'github:rust-lang:crates-io');
    await page.click('[data-test-save-button]');

    await expect(page.locator('[data-test-notification-message="success"]')).toHaveText(
      'Team github:rust-lang:crates-io was added as a crate owner',
    );
    await expect(page.locator('[data-test-owners] [data-test-owner-team]')).toHaveCount(3);
    await expect(page.locator('[data-test-owners] [data-test-owner-user]')).toHaveCount(2);
  });
});
