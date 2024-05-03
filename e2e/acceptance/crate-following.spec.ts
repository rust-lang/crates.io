import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | Crate following', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ mirage }) => {
    let hook = String(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.0' });

      let loggedIn = !globalThis.skipLogin;
      if (loggedIn) {
        let followedCrates = !!globalThis.following ? [crate] : [];
        let user = server.create('user', { followedCrates });
        globalThis.authenticateAs(user);
      }
    });
    await mirage.addHook(hook);
  });

  test("unauthenticated users don't see the follow button", async ({ page }) => {
    await page.addInitScript(() => {
      globalThis.skipLogin = true;
    });
    await page.goto('/crates/nanomsg');
    await expect(page.locator('[data-test-follow-button]')).toHaveCount(0);
  });

  test('authenticated users see a loading spinner and can follow/unfollow crates', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      globalThis.defer = require('rsvp').defer;
      globalThis.followingDeferred = globalThis.defer();
      server.get('/api/v1/crates/:crate_id/following', globalThis.followingDeferred.promise);
    });

    await page.goto('/crates/nanomsg');

    const followButton = page.locator('[data-test-follow-button]');
    const spinner = followButton.locator('[data-test-spinner]');
    await expect(followButton).toHaveText('Loading…');
    await expect(followButton).toBeDisabled();
    await expect(spinner).toBeVisible();

    await page.evaluate(() => globalThis.followingDeferred.resolve({ following: false }));
    await expect(followButton).toHaveText('Follow');
    await expect(followButton).toBeEnabled();
    await expect(spinner).toHaveCount(0);

    await page.evaluate(() => {
      globalThis.followDeferred = globalThis.defer();
      server.put('/api/v1/crates/:crate_id/follow', globalThis.followDeferred.promise);
    });
    await followButton.click();
    await expect(followButton).toHaveText('Loading…');
    await expect(followButton).toBeDisabled();
    await expect(spinner).toBeVisible();

    await page.evaluate(() => globalThis.followDeferred.resolve({ ok: true }));
    await expect(followButton).toHaveText('Unfollow');
    await expect(followButton).toBeEnabled();
    await expect(spinner).toHaveCount(0);

    await page.evaluate(() => {
      globalThis.unfollowDeferred = globalThis.defer();
      server.delete('/api/v1/crates/:crate_id/follow', globalThis.unfollowDeferred.promise);
    });
    await followButton.click();
    await expect(followButton).toHaveText('Loading…');
    await expect(followButton).toBeDisabled();
    await expect(spinner).toBeVisible();

    await page.evaluate(() => globalThis.unfollowDeferred.resolve({ ok: true }));
    await expect(followButton).toHaveText('Follow');
    await expect(followButton).toBeEnabled();
    await expect(spinner).toHaveCount(0);
  });

  test('error handling when loading following state fails', async ({ mirage, page }) => {
    await mirage.addHook(server => {
      server.get('/api/v1/crates/:crate_id/following', {}, 500);
    });

    await page.goto('/crates/nanomsg');
    const followButton = page.locator('[data-test-follow-button]');
    await expect(followButton).toHaveText('Follow');
    await expect(followButton).toBeDisabled();
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Something went wrong while trying to figure out if you are already following the nanomsg crate. Please try again later!',
    );
  });

  test('error handling when follow fails', async ({ mirage, page }) => {
    await mirage.addHook(server => {
      server.put('/api/v1/crates/:crate_id/follow', {}, 500);
    });

    await page.goto('/crates/nanomsg');
    await page.locator('[data-test-follow-button]').click();
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Something went wrong when following the nanomsg crate. Please try again later!',
    );
  });

  test('error handling when unfollow fails', async ({ mirage, page }) => {
    await page.addInitScript(() => {
      globalThis.following = true;
    });
    await mirage.addHook(server => {
      server.del('/api/v1/crates/:crate_id/follow', {}, 500);
    });

    await page.goto('/crates/nanomsg');
    await page.locator('[data-test-follow-button]').click();
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Something went wrong when unfollowing the nanomsg crate. Please try again later!',
    );
  });
});
