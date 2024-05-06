import { test, expect } from '@/e2e/helper';

test.describe('/settings/tokens', { tag: '@routes' }, () => {
  test.beforeEach(async ({ mirage }) => {
    await mirage.addHook(server => {
      let user = server.create('user', {
        login: 'johnnydee',
        name: 'John Doe',
        email: 'john@doe.com',
        avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      });

      authenticateAs(user);

      globalThis.user = user;
    });
  });

  test('reloads all tokens from the server', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      const user = globalThis.user;
      server.create('api-token', { user, name: 'token-1' });
    });

    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'token-2');
    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');

    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token]')).toHaveCount(2);
    let tokens = await page.locator('[data-test-api-token]').all();
    await expect(tokens[0].locator('[data-test-name]')).toHaveText('token-2');
    await expect(tokens[0].locator('[data-test-token]')).toBeVisible();
    await expect(tokens[1].locator('[data-test-name]')).toHaveText('token-1');
    await expect(tokens[1].locator('[data-test-token]')).toHaveCount(0);
  });
});
