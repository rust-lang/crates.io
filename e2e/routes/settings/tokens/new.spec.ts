import { expect, test } from '@/e2e/helper';
import { Response } from 'miragejs';

test.describe('/settings/tokens/new', { tag: '@routes' }, () => {
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

  test('can navigate to the route', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveURL('/');

    await page.click('[data-test-user-menu] [data-test-toggle]');
    await page.click('[data-test-user-menu] [data-test-settings]');
    await expect(page).toHaveURL('/settings/profile');

    await page.click('[data-test-settings-menu] [data-test-tokens] a');
    await expect(page).toHaveURL('/settings/tokens');

    await page.click('[data-test-new-token-button]');
    await expect(page).toHaveURL('/settings/tokens/new');
  });

  test('happy path', async ({ page }) => {
    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'token-name');
    await page.locator('[data-test-expiry]').selectOption('none');
    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');

    let token = await page.evaluate(() => {
      let token = server.schema['apiTokens'].findBy({ name: 'token-name' });
      return JSON.parse(JSON.stringify(token));
    });
    expect(token, 'API token has been created in the backend database').toBeTruthy();
    expect(token.name).toBe('token-name');
    expect(token.expiredAt).toBe(null);
    expect(token.crateScopes).toBe(null);
    expect(token.endpointScopes).toEqual(['publish-update']);

    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token="1"] [data-test-name]')).toHaveText('token-name');
    await expect(page.locator('[data-test-api-token="1"] [data-test-token]')).toHaveText(token.token);
    await expect(page.locator('[data-test-api-token="1"] [data-test-endpoint-scopes]')).toHaveText(
      'Scopes: publish-update',
    );
    await expect(page.locator('[data-test-api-token="1"] [data-test-crate-scopes]')).toHaveCount(0);
    await expect(page.locator('[data-test-api-token="1"] [data-test-expired-at]')).toHaveCount(0);
  });

  test('crate scopes', async ({ page }) => {
    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'token-name');
    await page.locator('[data-test-expiry]').selectOption('none');
    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-scope="yank"]');

    await expect(page.locator('[data-test-crates-unrestricted]')).toBeVisible();
    await expect(page.locator('[data-test-crate-pattern]')).toHaveCount(0);

    await page.click('[data-test-add-crate-pattern]');
    await expect(page.locator('[data-test-crates-unrestricted]')).toHaveCount(0);
    await expect(page.locator('[data-test-crate-pattern]')).toHaveCount(1);
    await expect(page.locator('[data-test-crate-pattern="0"] [data-test-description]')).toHaveText(
      'Please enter a crate name pattern',
    );

    await page.fill('[data-test-crate-pattern="0"] input', 'serde');
    await expect(page.locator('[data-test-crate-pattern="0"] [data-test-description]')).toHaveText(
      'Matches only the serde crate',
    );

    await page.click('[data-test-crate-pattern="0"] [data-test-remove]');
    await expect(page.locator('[data-test-crates-unrestricted]')).toBeVisible();
    await expect(page.locator('[data-test-crate-pattern]')).toHaveCount(0);

    await page.click('[data-test-add-crate-pattern]');
    await expect(page.locator('[data-test-crates-unrestricted]')).toHaveCount(0);
    await expect(page.locator('[data-test-crate-pattern]')).toHaveCount(1);
    await expect(page.locator('[data-test-crate-pattern="0"] [data-test-description]')).toHaveText(
      'Please enter a crate name pattern',
    );

    await page.fill('[data-test-crate-pattern="0"] input', 'serde-*');
    await expect(page.locator('[data-test-crate-pattern="0"] [data-test-description]')).toHaveText(
      'Matches all crates starting with serde-',
    );

    await page.click('[data-test-add-crate-pattern]');
    await expect(page.locator('[data-test-crates-unrestricted]')).toHaveCount(0);
    await expect(page.locator('[data-test-crate-pattern]')).toHaveCount(2);
    await expect(page.locator('[data-test-crate-pattern="1"] [data-test-description]')).toHaveText(
      'Please enter a crate name pattern',
    );

    await page.fill('[data-test-crate-pattern="1"] input', 'inv@lid');
    await expect(page.locator('[data-test-crate-pattern="1"] [data-test-description]')).toHaveText(
      'Invalid crate name pattern',
    );

    await page.click('[data-test-add-crate-pattern]');
    await expect(page.locator('[data-test-crates-unrestricted]')).toHaveCount(0);
    await expect(page.locator('[data-test-crate-pattern]')).toHaveCount(3);
    await expect(page.locator('[data-test-crate-pattern="2"] [data-test-description]')).toHaveText(
      'Please enter a crate name pattern',
    );

    await page.fill('[data-test-crate-pattern="2"] input', 'serde');
    await expect(page.locator('[data-test-crate-pattern="2"] [data-test-description]')).toHaveText(
      'Matches only the serde crate',
    );

    await page.click('[data-test-crate-pattern="1"] [data-test-remove]');
    await expect(page.locator('[data-test-crates-unrestricted]')).toHaveCount(0);
    await expect(page.locator('[data-test-crate-pattern]')).toHaveCount(2);

    await page.click('[data-test-generate]');

    let token = await page.evaluate(() => {
      let token = server.schema['apiTokens'].findBy({ name: 'token-name' });
      return JSON.parse(JSON.stringify(token));
    });
    expect(token, 'API token has been created in the backend database').toBeTruthy();
    expect(token.name).toBe('token-name');
    expect(token.crateScopes).toEqual(['serde-*', 'serde']);
    expect(token.endpointScopes).toEqual(['publish-update', 'yank']);

    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token="1"] [data-test-name]')).toHaveText('token-name');
    await expect(page.locator('[data-test-api-token="1"] [data-test-token]')).toHaveText(token.token);
    await expect(page.locator('[data-test-api-token="1"] [data-test-endpoint-scopes]')).toHaveText(
      'Scopes: publish-update and yank',
    );
    await expect(page.locator('[data-test-api-token="1"] [data-test-crate-scopes]')).toHaveText(
      'Crates: serde-* and serde',
    );
    await expect(page.locator('[data-test-api-token="1"] [data-test-expired-at]')).toHaveCount(0);
  });

  test('token expiry', async ({ page }) => {
    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');
    await expect(page.locator('[data-test-name]')).toHaveValue('');
    await expect(page.locator('[data-test-expiry]')).toHaveValue('90');
    let expiryDate = new Date('2018-02-18T00:00:00');
    let expectedDate = expiryDate.toLocaleDateString(undefined, { dateStyle: 'long' });
    let expectedDescription = `The token will expire on ${expectedDate}`;
    await expect(page.locator('[data-test-expiry-description]')).toHaveText(expectedDescription);

    await page.fill('[data-test-name]', 'token-name');
    await page.locator('[data-test-expiry]').selectOption('none');
    await expect(page.locator('[data-test-expiry-description]')).toHaveText('The token will never expire');

    await page.locator('[data-test-expiry]').selectOption('30');
    expiryDate = new Date('2017-12-20T00:00:00');
    expectedDate = expiryDate.toLocaleDateString(undefined, { dateStyle: 'long' });
    expectedDescription = `The token will expire on ${expectedDate}`;
    await expect(page.locator('[data-test-expiry-description]')).toHaveText(expectedDescription);

    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');

    let token = await page.evaluate(() => {
      let token = server.schema['apiTokens'].findBy({ name: 'token-name' });
      return JSON.parse(JSON.stringify(token));
    });
    expect(token, 'API token has been created in the backend database').toBeTruthy();
    expect(token.name).toBe('token-name');
    expect(token.expiredAt.slice(0, 10)).toBe('2017-12-20');
    expect(token.crateScopes).toBe(null);
    expect(token.endpointScopes).toEqual(['publish-update']);

    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token="1"] [data-test-name]')).toHaveText('token-name');
    await expect(page.locator('[data-test-api-token="1"] [data-test-token]')).toHaveText(token.token);
    await expect(page.locator('[data-test-api-token="1"] [data-test-endpoint-scopes]')).toHaveText(
      'Scopes: publish-update',
    );
    await expect(page.locator('[data-test-api-token="1"] [data-test-crate-scopes]')).toHaveCount(0);
    await expect(page.locator('[data-test-api-token="1"] [data-test-expired-at]')).toHaveText(
      'Expires in about 1 month',
    );
  });

  test('token expiry with custom date', async ({ page }) => {
    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'token-name');
    await page.locator('[data-test-expiry]').selectOption('none');
    await expect(page.locator('[data-test-expiry-description]')).toHaveText('The token will never expire');
    await page.locator('[data-test-expiry]').selectOption('custom');
    await expect(page.locator('[data-test-expiry-description]')).toHaveCount(0);

    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');
    await expect(page.locator('[data-test-expiry-date]')).toHaveAttribute('aria-invalid', 'true');

    await page.fill('[data-test-expiry-date]', '2024-05-04');
    await expect(page.locator('[data-test-expiry-description]')).toHaveCount(0);

    await page.click('[data-test-generate]');

    let token = await page.evaluate(() => {
      let token = server.schema['apiTokens'].findBy({ name: 'token-name' });
      return JSON.parse(JSON.stringify(token));
    });
    expect(token, 'API token has been created in the backend database').toBeTruthy();
    expect(token.name).toBe('token-name');
    expect(token.expiredAt.slice(0, 10)).toBe('2024-05-04');
    expect(token.crateScopes).toBe(null);
    expect(token.endpointScopes).toEqual(['publish-update']);

    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token="1"] [data-test-name]')).toHaveText('token-name');
    await expect(page.locator('[data-test-api-token="1"] [data-test-token]')).toHaveText(token.token);
    await expect(page.locator('[data-test-api-token="1"] [data-test-endpoint-scopes]')).toHaveText(
      'Scopes: publish-update',
    );
    await expect(page.locator('[data-test-api-token="1"] [data-test-crate-scopes]')).toHaveCount(0);
    await expect(page.locator('[data-test-api-token="1"] [data-test-expired-at]')).toHaveText(
      'Expires in over 6 years',
    );
  });

  test('loading and error state', async ({ page, mirage }) => {
    await page.exposeBinding('resp500', () => new Response(500));
    await mirage.addHook(server => {
      globalThis.deferred = require('rsvp').defer();
      server.put('/api/v1/me/tokens', () => globalThis.deferred.promise);
    });

    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'token-name');
    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');
    await expect(page.locator('[data-test-generate] [data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-name]')).toBeDisabled();
    await expect(page.locator('[data-test-generate]')).toBeDisabled();

    await page.evaluate(async () => globalThis.deferred.resolve(await globalThis.resp500));

    let message = 'An error has occurred while generating your API token. Please try again later!';
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(message);
    await expect(page.locator('[data-test-name]')).toBeEnabled();
    await expect(page.locator('[data-test-generate]')).toBeEnabled();
  });

  test('cancel button navigates back to the token list', async ({ page }) => {
    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.click('[data-test-cancel]');
    await expect(page).toHaveURL('/settings/tokens');
  });

  test('empty name shows an error', async ({ page }) => {
    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');
    await expect(page).toHaveURL('/settings/tokens/new');
    await expect(page.locator('[data-test-name]')).toHaveAttribute('aria-invalid', 'true');
    await expect(page.locator('[data-test-name-group] [data-test-error]')).toBeVisible();
    await expect(page.locator('[data-test-scopes-group] [data-test-error]')).toHaveCount(0);
  });

  test('no scopes selected shows an error', async ({ page }) => {
    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'token-name');
    await page.click('[data-test-generate]');
    await expect(page).toHaveURL('/settings/tokens/new');
    await expect(page.locator('[data-test-name-group] [data-test-error]')).toHaveCount(0);
    await expect(page.locator('[data-test-scopes-group] [data-test-error]')).toBeVisible();
  });

  test('prefill with the exist token', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      const user = globalThis.user;

      server.create('apiToken', {
        user: user,
        id: '1',
        name: 'foo',
        token: 'test',
        createdAt: '2017-08-01T12:34:56',
        lastUsedAt: '2017-11-02T01:45:14',
        endpointScopes: ['publish-update'],
      });
    });

    await page.goto('/settings/tokens/new?from=1');
    await expect(page).toHaveURL('/settings/tokens/new?from=1');
    await expect(page.locator('[data-test-crates-unrestricted]')).toBeVisible();
    await expect(page.locator('[data-test-crate-pattern]')).toHaveCount(0);

    await page.click('[data-test-add-crate-pattern]');
    await expect(page.locator('[data-test-crates-unrestricted]')).toHaveCount(0);
    await expect(page.locator('[data-test-crate-pattern]')).toHaveCount(1);

    await page.fill('[data-test-crate-pattern="0"] input', 'serde');
    await expect(page.locator('[data-test-crate-pattern="0"] [data-test-description]')).toHaveText(
      'Matches only the serde crate',
    );
    await page.click('[data-test-generate]');

    let newToken = await page.evaluate(() => {
      let newToken = server.schema['apiTokens'].findBy({ name: 'foo', crateScopes: ['serde'] });
      return JSON.parse(JSON.stringify(newToken));
    });
    expect(newToken, 'New API token has been created in the backend database').toBeTruthy();

    await expect(page).toHaveURL('/settings/tokens');
    await page.click('[data-test-new-token-button]');
    // It should reset the token ID query parameter.
    await expect(page).toHaveURL('/settings/tokens/new');
  });

  test('token not found', async ({ page }) => {
    await page.goto('/settings/tokens/new?from=1');
    await expect(page).toHaveURL('/settings/tokens/new?from=1');
    await expect(page.locator('[data-test-title]')).toHaveText('Token not found');
  });
});

test.describe('/settings/tokens/new', { tag: '@routes' }, () => {
  test('access is blocked if unauthenticated', async ({ page }) => {
    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });
});
