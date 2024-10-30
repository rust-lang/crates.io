import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | Settings | Add Owner', { tag: '@acceptance' }, () => {
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

  test('add a new owner', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.create('user', { name: 'iain8' });
    });

    await page.goto('/crates/nanomsg/settings');
    await page.fill('input[name="username"]', 'iain8');
    await page.click('[data-test-save-button]');

    await expect(page.locator('[data-test-notification-message="success"]')).toHaveText(
      'An invite has been sent to iain8',
    );
    await expect(page.locator('[data-test-owners] [data-test-owner-team]')).toHaveCount(2);
    await expect(page.locator('[data-test-owners] [data-test-owner-user]')).toHaveCount(2);
  });

  test('add a team owner', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.create('user', { name: 'iain8' });
      server.create('team', { org: 'rust-lang', name: 'crates-io' });
    });

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
