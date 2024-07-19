import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | api-tokens', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ mirage }) => {
    await mirage.addHook(server => {
      let user = server.create('user', {
        login: 'johnnydee',
        name: 'John Doe',
        email: 'john@doe.com',
        avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      });
      server.create('api-token', {
        user,
        name: 'BAR',
        createdAt: '2017-11-19T17:59:22',
        lastUsedAt: null,
        expiredAt: '2017-12-19T17:59:22',
      });

      server.create('api-token', {
        user,
        name: 'recently expired',
        createdAt: '2017-08-01T12:34:56',
        lastUsedAt: '2017-11-02T01:45:14',
        expiredAt: '2017-11-19T17:59:22',
      });
      server.create('api-token', {
        user,
        name: 'foo',
        createdAt: '2017-08-01T12:34:56',
        lastUsedAt: '2017-11-02T01:45:14',
      });

      globalThis.authenticateAs(user);
    });
  });

  test('/me is showing the list of active API tokens', async ({ page }) => {
    await page.goto('/settings/tokens');
    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token]')).toHaveCount(3);

    let [row1, row2, row3] = await page.locator('[data-test-api-token]').all();
    await expect(row1.locator('[data-test-name]')).toHaveText('BAR');
    await expect(row1.locator('[data-test-created-at]')).toHaveText('Created about 18 hours ago');
    await expect(row1.locator('[data-test-last-used-at]')).toHaveText('Never used');
    await expect(row1.locator('[data-test-expired-at]')).toHaveText('Expires in 29 days');
    await expect(row1.locator('[data-test-save-token-button]')).toHaveCount(0);
    await expect(row1.locator('[data-test-revoke-token-button]')).toBeVisible();
    await expect(row1.locator('[data-test-saving-spinner]')).toHaveCount(0);
    await expect(row1.locator('[data-test-error]')).toHaveCount(0);
    await expect(row1.locator('[data-test-token]')).toHaveCount(0);

    await expect(row2.locator('[data-test-name]')).toHaveText('foo');
    await expect(row2.locator('[data-test-created-at]')).toHaveText('Created 4 months ago');
    await expect(row2.locator('[data-test-last-used-at]')).toHaveText('Last used 18 days ago');
    await expect(row2.locator('[data-test-expired-at]')).toHaveCount(0);
    await expect(row2.locator('[data-test-save-token-button]')).toHaveCount(0);
    await expect(row2.locator('[data-test-revoke-token-button]')).toBeVisible();
    await expect(row2.locator('[data-test-saving-spinner]')).toHaveCount(0);
    await expect(row2.locator('[data-test-error]')).toHaveCount(0);
    await expect(row2.locator('[data-test-token]')).toHaveCount(0);

    await expect(row3.locator('[data-test-name]')).toHaveText('recently expired');
    await expect(row3.locator('[data-test-created-at]')).toHaveText('Created 4 months ago');
    await expect(row3.locator('[data-test-last-used-at]')).toHaveText('Last used 18 days ago');
    await expect(row3.locator('[data-test-expired-at]')).toHaveText('Expired about 18 hours ago');
    await expect(row3.locator('[data-test-save-token-button]')).toHaveCount(0);
    await expect(row3.locator('[data-test-revoke-token-button]')).toHaveCount(0);
    await expect(row3.locator('[data-test-saving-spinner]')).toHaveCount(0);
    await expect(row3.locator('[data-test-error]')).toHaveCount(0);
    await expect(row3.locator('[data-test-token]')).toHaveCount(0);
  });

  test('API tokens can be revoked', async ({ page }) => {
    await page.goto('/settings/tokens');
    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token]')).toHaveCount(3);

    await page.click('[data-test-api-token="1"] [data-test-revoke-token-button]');
    expect(
      await page.evaluate(() => server.schema['apiTokens'].all().length),
      'API token has been deleted from the backend database',
    ).toBe(2);

    await expect(page.locator('[data-test-api-token]')).toHaveCount(2);
    await expect(page.locator('[data-test-api-token="2"]')).toBeVisible();
    await expect(page.locator('[data-test-error]')).toHaveCount(0);
  });

  test('API tokens can be regenerated', async ({ page }) => {
    await page.goto('/settings/tokens');
    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token]')).toHaveCount(3);

    await page.click('[data-test-api-token="1"] [data-test-regenerate-token-button]');
    await expect(page).toHaveURL('/settings/tokens/new?from=1');
  });

  test('failed API tokens revocation shows an error', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.delete('/api/v1/me/tokens/:id', {}, 500);
    });

    await mirage.page.goto('/settings/tokens');
    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token]')).toHaveCount(3);

    await page.click('[data-test-api-token="1"] [data-test-revoke-token-button]');
    await expect(page.locator('[data-test-api-token]')).toHaveCount(3);
    await expect(page.locator('[data-test-api-token="2"]')).toBeVisible();
    await expect(page.locator('[data-test-api-token="1"]')).toBeVisible();
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'An unknown error occurred while revoking this token',
    );
  });

  test('new API tokens can be created', async ({ page, percy }) => {
    await page.goto('/settings/tokens');
    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token]')).toHaveCount(3);

    await page.click('[data-test-new-token-button]');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'the new token');
    await page.click('[data-test-scope="publish-update"]');
    await percy.snapshot();

    await page.click('[data-test-generate]');

    let token = await page.evaluate(() => server.schema['apiTokens'].findBy({ name: 'the new token' })?.token);
    expect(token, 'API token has been created in the backend database').toBeTruthy();

    await expect(page.locator('[data-test-api-token="4"] [data-test-name]')).toHaveText('the new token');
    await expect(page.locator('[data-test-api-token="4"] [data-test-save-token-button]')).toHaveCount(0);
    await expect(page.locator('[data-test-api-token="4"] [data-test-revoke-token-button]')).toBeVisible();
    await expect(page.locator('[data-test-api-token="4"] [data-test-saving-spinner]')).toHaveCount(0);
    await expect(page.locator('[data-test-api-token="4"] [data-test-error]')).toHaveCount(0);
    await expect(page.locator('[data-test-token]')).toHaveText(token);
  });

  test('API tokens are only visible in plaintext until the page is left', async ({ page }) => {
    await page.goto('/settings/tokens');
    await page.click('[data-test-new-token-button]');
    await page.fill('[data-test-name]', 'the new token');
    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');

    let token = await page.evaluate(() => server.schema['apiTokens'].findBy({ name: 'the new token' })?.token);
    await expect(page.locator('[data-test-token]')).toHaveText(token);

    // leave the API tokens page
    // favor navigation via link click over page.goto
    await page.getByRole('link', { name: 'Profile' }).click();
    await expect(page).toHaveURL('/settings/profile');

    // and visit it again
    // favor navigation via link click over page.goto
    await page.getByRole('link', { name: 'API Tokens' }).click();
    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-token]')).toHaveCount(0);
  });

  test('navigating away while creating a token does not keep it in the list', async ({ page }) => {
    await page.goto('/settings/tokens');
    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token]')).toHaveCount(3);

    await page.click('[data-test-new-token-button]');

    // favor navigation via link click over page.goto
    await page.getByRole('link', { name: 'Profile' }).click();
    await expect(page).toHaveURL('/settings/profile');

    // favor navigation via link click over page.goto
    await page.getByRole('link', { name: 'API Tokens' }).click();
    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token]')).toHaveCount(3);
  });
});
