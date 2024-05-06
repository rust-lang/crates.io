import { test, expect } from '@/e2e/helper';

test.describe('Bug #2329', { tag: '@bugs' }, () => {
  test('is fixed', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let user = server.create('user');

      let foobar = server.create('crate', { name: 'foo-bar' });
      server.create('crate-ownership', { crate: foobar, user, emailNotifications: true });
      server.create('version', { crate: foobar });

      let bar = server.create('crate', { name: 'barrrrr' });
      server.create('crate-ownership', { crate: bar, user, emailNotifications: false });
      server.create('version', { crate: bar });

      server.get('/api/private/session/begin', { url: 'url-to-github-including-state-secret' });

      server.get('/api/private/session/authorize', () => {
        authenticateAs(user);
        return { ok: true };
      });
    });

    await page.addInitScript(() => {
      let fakeWindow = { document: { write() {}, close() {} }, close() {} };
      window.open = (() => fakeWindow) as typeof open;
    });

    // 1. Sign out.
    // ...

    // 2. Open the network debug panel.
    // ...

    // 3. Refresh the page.
    await page.goto('/');

    // 4. Click the "Log in with GitHub" link.
    await page.click('[data-test-login-button]');

    // 5. Complete the authentication workflow if necessary.

    // simulate the response from the `github-authorize` route
    await page.evaluate(() => {
      window.postMessage({ code: 'foo', state: 'bar' }, window.location.origin);
    });

    // 6. Use the menu to navigate to "Account Settings".
    await page.click('[data-test-user-menu]');
    await page.getByRole('link', { name: 'Account Settings' }).click();
    // Instead of using goto for navigation, we use clicks. Since DOMContentLoaded fires only once
    // in SPA, this approach eliminates the need for repeated resource fetching and maintains the
    // logged-in state.
    await page.getByRole('link', { name: 'Email Notifications' }).click();
    await expect(page).toHaveURL('/settings/email-notifications');

    // // 7. Observe that there are no crates listed under "Email Notification Preferences".
    // // Observe that no request was sent to the /api/v1/me endpoint to obtain this data.
    //
    // // here we divert from the reproduction instructions, since this is the
    // // issue that we want to fix
    await expect(page.locator('[data-test-owned-crate]')).toHaveCount(2);
  });
});
