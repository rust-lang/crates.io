import { defer } from '@/e2e/deferred';
import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | Crate following', { tag: '@acceptance' }, () => {
  async function prepare(msw, { skipLogin = false, following = false } = {}) {
    let crate = msw.db.crate.create({ name: 'nanomsg' });
    msw.db.version.create({ crate, num: '0.6.0' });

    let loggedIn = !skipLogin;
    if (loggedIn) {
      let followedCrates = following ? [crate] : [];
      let user = msw.db.user.create({ followedCrates });
      await msw.authenticateAs(user);
    }
  }

  test("unauthenticated users don't see the follow button", async ({ page, msw }) => {
    await prepare(msw, { skipLogin: true });

    await page.goto('/crates/nanomsg');
    await expect(page.locator('[data-test-follow-button]')).toHaveCount(0);
  });

  test('authenticated users see a loading spinner and can follow/unfollow crates', async ({ page, msw }) => {
    await prepare(msw);

    let followingDeferred = defer();
    await msw.worker.use(http.get('/api/v1/crates/:crate_id/following', () => followingDeferred.promise));

    await page.goto('/crates/nanomsg');

    const followButton = page.locator('[data-test-follow-button]');
    const spinner = followButton.locator('[data-test-spinner]');
    await expect(followButton).toHaveText('Loading…');
    await expect(followButton).toBeDisabled();
    await expect(spinner).toBeVisible();

    followingDeferred.resolve(HttpResponse.json({ following: false }));
    await expect(followButton).toHaveText('Follow');
    await expect(followButton).toBeEnabled();
    await expect(spinner).toHaveCount(0);

    let followDeferred = defer();
    await msw.worker.use(http.put('/api/v1/crates/:crate_id/follow', () => followDeferred.promise));
    await followButton.click();
    await expect(followButton).toHaveText('Loading…');
    await expect(followButton).toBeDisabled();
    await expect(spinner).toBeVisible();

    followDeferred.resolve(HttpResponse.json({ ok: true }));
    await expect(followButton).toHaveText('Unfollow');
    await expect(followButton).toBeEnabled();
    await expect(spinner).toHaveCount(0);

    let unfollowDeferred = defer();
    await msw.worker.use(http.delete('/api/v1/crates/:crate_id/follow', () => unfollowDeferred.promise));
    await followButton.click();
    await expect(followButton).toHaveText('Loading…');
    await expect(followButton).toBeDisabled();
    await expect(spinner).toBeVisible();

    unfollowDeferred.resolve(HttpResponse.json({ ok: true }));
    await expect(followButton).toHaveText('Follow');
    await expect(followButton).toBeEnabled();
    await expect(spinner).toHaveCount(0);
  });

  test('error handling when loading following state fails', async ({ msw, page }) => {
    await prepare(msw);

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.get('/api/v1/crates/:crate_id/following', () => error));

    await page.goto('/crates/nanomsg');
    const followButton = page.locator('[data-test-follow-button]');
    await expect(followButton).toHaveText('Follow');
    await expect(followButton).toBeDisabled();
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Something went wrong while trying to figure out if you are already following the nanomsg crate. Please try again later!',
    );
  });

  test('error handling when follow fails', async ({ msw, page }) => {
    await prepare(msw);

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.put('/api/v1/crates/:crate_id/follow', () => error));

    await page.goto('/crates/nanomsg');
    await page.locator('[data-test-follow-button]').click();
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Something went wrong when following the nanomsg crate. Please try again later!',
    );
  });

  test('error handling when unfollow fails', async ({ msw, page }) => {
    await prepare(msw, { following: true });

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.delete('/api/v1/crates/:crate_id/follow', () => error));

    await page.goto('/crates/nanomsg');
    await page.locator('[data-test-follow-button]').click();
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Something went wrong when unfollowing the nanomsg crate. Please try again later!',
    );
  });
});
