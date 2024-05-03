import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | Dashboard', { tag: '@acceptance' }, () => {
  test('shows "page requires authentication" error when not logged in', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page).toHaveURL('/dashboard');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });

  test('shows the dashboard when logged in', async ({ page, mirage, percy }) => {
    await mirage.addHook(server => {
      let user = server.create('user', {
        login: 'johnnydee',
        name: 'John Doe',
        email: 'john@doe.com',
        avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      });

      authenticateAs(user);

      {
        let crate = server.create('crate', { name: 'rand' });
        server.create('version', { crate, num: '0.5.0' });
        server.create('version', { crate, num: '0.6.0' });
        server.create('version', { crate, num: '0.7.0' });
        server.create('version', { crate, num: '0.7.1' });
        server.create('version', { crate, num: '0.7.2' });
        server.create('version', { crate, num: '0.7.3' });
        server.create('version', { crate, num: '0.8.0' });
        server.create('version', { crate, num: '0.8.1' });
        server.create('version', { crate, num: '0.9.0' });
        server.create('version', { crate, num: '1.0.0' });
        server.create('version', { crate, num: '1.1.0' });
        user.followedCrates.add(crate);
      }

      {
        let crate = server.create('crate', { name: 'nanomsg' });
        server.create('crate-ownership', { crate, user });
        server.create('version', { crate, num: '0.1.0' });
        user.followedCrates.add(crate);
      }

      user.save();

      server.get(`/api/v1/users/${user.id}/stats`, { total_downloads: 3892 });
    });

    await page.goto('/dashboard');
    await expect(page).toHaveURL('/dashboard');
    await percy.snapshot();
  });
});
