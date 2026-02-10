import { defer } from '@/e2e/deferred';
import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('/settings/tokens/new', { tag: '@routes' }, () => {
  async function prepare(msw) {
    let user = await msw.db.user.create({
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });

    await msw.authenticateAs(user);

    return { user };
  }

  test('can navigate to the route', async ({ page, msw }) => {
    await prepare(msw);

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

  test('happy path', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'token-name');
    await page.locator('[data-test-expiry]').selectOption('none');
    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');

    let token = msw.db.apiToken.findFirst(q => q.where({ name: 'token-name' }));
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

  test('crate scopes', async ({ page, msw }) => {
    await prepare(msw);

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

    let token = msw.db.apiToken.findFirst(q => q.where({ name: 'token-name' }));
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

  test('token expiry', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');
    await expect(page.locator('[data-test-name]')).toHaveValue('');
    await expect(page.locator('[data-test-expiry]')).toHaveValue('90');
    let expectedDescription = `The token will expire on February 18, 2018`;
    await expect(page.locator('[data-test-expiry-description]')).toHaveText(expectedDescription);

    await page.fill('[data-test-name]', 'token-name');
    await page.locator('[data-test-expiry]').selectOption('none');
    await expect(page.locator('[data-test-expiry-description]')).toHaveText('The token will never expire');

    await page.locator('[data-test-expiry]').selectOption('30');
    expectedDescription = `The token will expire on December 20, 2017`;
    await expect(page.locator('[data-test-expiry-description]')).toHaveText(expectedDescription);

    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');

    let token = msw.db.apiToken.findFirst(q => q.where({ name: 'token-name' }));
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

  test('token expiry with custom date', async ({ page, msw }) => {
    await prepare(msw);

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

    let token = msw.db.apiToken.findFirst(q => q.where({ name: 'token-name' }));
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

  test('loading and error state', async ({ page, msw }) => {
    await prepare(msw);

    let deferred = defer();
    await msw.worker.use(http.put('/api/v1/me/tokens', () => deferred.promise));

    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'token-name');
    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');
    await expect(page.locator('[data-test-generate] [data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-name]')).toBeDisabled();
    await expect(page.locator('[data-test-generate]')).toBeDisabled();

    deferred.resolve(HttpResponse.json({}, { status: 500 }));

    let message = 'An error has occurred while generating your API token. Please try again later!';
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(message);
    await expect(page.locator('[data-test-name]')).toBeEnabled();
    await expect(page.locator('[data-test-generate]')).toBeEnabled();
  });

  test('cancel button navigates back to the token list', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.click('[data-test-cancel]');
    await expect(page).toHaveURL('/settings/tokens');
  });

  test('empty name shows an error', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.click('[data-test-scope="publish-update"]');
    await page.click('[data-test-generate]');
    await expect(page).toHaveURL('/settings/tokens/new');
    await expect(page.locator('[data-test-name]')).toHaveAttribute('aria-invalid', 'true');
    await expect(page.locator('[data-test-name-group] [data-test-error]')).toBeVisible();
    await expect(page.locator('[data-test-scopes-group] [data-test-error]')).toHaveCount(0);
  });

  test('no scopes selected shows an error', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'token-name');
    await page.click('[data-test-generate]');
    await expect(page).toHaveURL('/settings/tokens/new');
    await expect(page.locator('[data-test-name-group] [data-test-error]')).toHaveCount(0);
    await expect(page.locator('[data-test-scopes-group] [data-test-error]')).toBeVisible();
  });

  test('prefill with the exist token', async ({ page, msw }) => {
    let { user } = await prepare(msw);

    await msw.db.apiToken.create({
      user: user,
      id: 1,
      name: 'foo',
      token: 'test',
      createdAt: '2017-08-01T12:34:56',
      lastUsedAt: '2017-11-02T01:45:14',
      endpointScopes: ['publish-update'],
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

    let newToken = msw.db.apiToken.findFirst(q => q.where({ name: 'foo' }));
    expect(newToken, 'New API token has been created in the backend database').toBeTruthy();

    await expect(page).toHaveURL('/settings/tokens');
    await page.click('[data-test-new-token-button]');
    // It should reset the token ID query parameter.
    await expect(page).toHaveURL('/settings/tokens/new');
  });

  test('token not found', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/settings/tokens/new?from=1');
    await expect(page).toHaveURL('/settings/tokens/new?from=1');
    await expect(page.locator('[data-test-title]')).toHaveText('Token not found');
  });

  test('trusted-publishing scope', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');

    await page.fill('[data-test-name]', 'trusted-publishing-token');
    await page.locator('[data-test-expiry]').selectOption('none');
    await page.click('[data-test-scope="trusted-publishing"]');
    await page.click('[data-test-generate]');

    let token = msw.db.apiToken.findFirst(q => q.where({ name: 'trusted-publishing-token' }));
    expect(token, 'API token has been created in the backend database').toBeTruthy();
    expect(token.name).toBe('trusted-publishing-token');
    expect(token.expiredAt).toBe(null);
    expect(token.crateScopes).toBe(null);
    expect(token.endpointScopes).toEqual(['trusted-publishing']);

    await expect(page).toHaveURL('/settings/tokens');
    await expect(page.locator('[data-test-api-token="1"] [data-test-name]')).toHaveText('trusted-publishing-token');
    await expect(page.locator('[data-test-api-token="1"] [data-test-token]')).toHaveText(token.token);
    await expect(page.locator('[data-test-api-token="1"] [data-test-endpoint-scopes]')).toHaveText(
      'Scopes: trusted-publishing',
    );
    await expect(page.locator('[data-test-api-token="1"] [data-test-crate-scopes]')).toHaveCount(0);
    await expect(page.locator('[data-test-api-token="1"] [data-test-expired-at]')).toHaveCount(0);
  });

  test('access is blocked if unauthenticated', async ({ page }) => {
    await page.goto('/settings/tokens/new');
    await expect(page).toHaveURL('/settings/tokens/new');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });
});
