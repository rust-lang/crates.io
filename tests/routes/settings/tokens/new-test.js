import { click, currentURL, fillIn, select, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../../helpers/visit-ignoring-abort';

module('/settings/tokens/new', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let user = context.db.user.create({
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });

    context.authenticateAs(user);

    return { user };
  }

  test('can navigate to the route', async function (assert) {
    prepare(this);

    await visit('/');
    assert.strictEqual(currentURL(), '/');

    await click('[data-test-user-menu] [data-test-toggle]');
    await click('[data-test-user-menu] [data-test-settings]');
    assert.strictEqual(currentURL(), '/settings/profile');

    await click('[data-test-settings-menu] [data-test-tokens] a');
    assert.strictEqual(currentURL(), '/settings/tokens');

    await click('[data-test-new-token-button]');
    assert.strictEqual(currentURL(), '/settings/tokens/new');
  });

  test('access is blocked if unauthenticated', async function (assert) {
    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('happy path', async function (assert) {
    prepare(this);

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await fillIn('[data-test-name]', 'token-name');
    await select('[data-test-expiry]', 'none');
    await click('[data-test-scope="publish-update"]');
    await click('[data-test-generate]');

    let token = this.db.apiToken.findFirst({ where: { name: { equals: 'token-name' } } });
    assert.ok(Boolean(token), 'API token has been created in the backend database');
    assert.strictEqual(token.name, 'token-name');
    assert.strictEqual(token.expiredAt, null);
    assert.strictEqual(token.crateScopes, null);
    assert.deepEqual(token.endpointScopes, ['publish-update']);

    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token="1"] [data-test-name]').hasText('token-name');
    assert.dom('[data-test-api-token="1"] [data-test-token]').hasText(token.token);
    assert.dom('[data-test-api-token="1"] [data-test-endpoint-scopes]').hasText('Scopes: publish-update');
    assert.dom('[data-test-api-token="1"] [data-test-crate-scopes]').doesNotExist();
    assert.dom('[data-test-api-token="1"] [data-test-expired-at]').doesNotExist();
  });

  test('crate scopes', async function (assert) {
    prepare(this);

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await fillIn('[data-test-name]', 'token-name');
    await select('[data-test-expiry]', 'none');
    await click('[data-test-scope="publish-update"]');
    await click('[data-test-scope="yank"]');

    assert.dom('[data-test-crates-unrestricted]').exists();
    assert.dom('[data-test-crate-pattern]').doesNotExist();

    await click('[data-test-add-crate-pattern]');
    assert.dom('[data-test-crates-unrestricted]').doesNotExist();
    assert.dom('[data-test-crate-pattern]').exists({ count: 1 });
    assert.dom('[data-test-crate-pattern="0"] [data-test-description]').hasText('Please enter a crate name pattern');

    await fillIn('[data-test-crate-pattern="0"] input', 'serde');
    assert.dom('[data-test-crate-pattern="0"] [data-test-description]').hasText('Matches only the serde crate');

    await click('[data-test-crate-pattern="0"] [data-test-remove]');
    assert.dom('[data-test-crates-unrestricted]').exists();
    assert.dom('[data-test-crate-pattern]').doesNotExist();

    await click('[data-test-add-crate-pattern]');
    assert.dom('[data-test-crates-unrestricted]').doesNotExist();
    assert.dom('[data-test-crate-pattern]').exists({ count: 1 });
    assert.dom('[data-test-crate-pattern="0"] [data-test-description]').hasText('Please enter a crate name pattern');

    await fillIn('[data-test-crate-pattern="0"] input', 'serde-*');
    assert
      .dom('[data-test-crate-pattern="0"] [data-test-description]')
      .hasText('Matches all crates starting with serde-');

    await click('[data-test-add-crate-pattern]');
    assert.dom('[data-test-crates-unrestricted]').doesNotExist();
    assert.dom('[data-test-crate-pattern]').exists({ count: 2 });
    assert.dom('[data-test-crate-pattern="1"] [data-test-description]').hasText('Please enter a crate name pattern');

    await fillIn('[data-test-crate-pattern="1"] input', 'inv@lid');
    assert.dom('[data-test-crate-pattern="1"] [data-test-description]').hasText('Invalid crate name pattern');

    await click('[data-test-add-crate-pattern]');
    assert.dom('[data-test-crates-unrestricted]').doesNotExist();
    assert.dom('[data-test-crate-pattern]').exists({ count: 3 });
    assert.dom('[data-test-crate-pattern="2"] [data-test-description]').hasText('Please enter a crate name pattern');

    await fillIn('[data-test-crate-pattern="2"] input', 'serde');
    assert.dom('[data-test-crate-pattern="2"] [data-test-description]').hasText('Matches only the serde crate');

    await click('[data-test-crate-pattern="1"] [data-test-remove]');
    assert.dom('[data-test-crates-unrestricted]').doesNotExist();
    assert.dom('[data-test-crate-pattern]').exists({ count: 2 });

    await click('[data-test-generate]');

    let token = this.db.apiToken.findFirst({ where: { name: { equals: 'token-name' } } });
    assert.ok(Boolean(token), 'API token has been created in the backend database');
    assert.strictEqual(token.name, 'token-name');
    assert.deepEqual(token.crateScopes, ['serde-*', 'serde']);
    assert.deepEqual(token.endpointScopes, ['publish-update', 'yank']);

    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token="1"] [data-test-name]').hasText('token-name');
    assert.dom('[data-test-api-token="1"] [data-test-token]').hasText(token.token);
    assert.dom('[data-test-api-token="1"] [data-test-endpoint-scopes]').hasText('Scopes: publish-update and yank');
    assert.dom('[data-test-api-token="1"] [data-test-crate-scopes]').hasText('Crates: serde-* and serde');
    assert.dom('[data-test-api-token="1"] [data-test-expired-at]').doesNotExist();
  });

  test('token expiry', async function (assert) {
    prepare(this);

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');
    assert.dom('[data-test-name]').hasValue('');
    assert.dom('[data-test-expiry]').hasValue('90');
    let expiryDate = new Date('2018-02-18T00:00:00');
    let expectedDate = expiryDate.toLocaleDateString(undefined, { dateStyle: 'long' });
    let expectedDescription = `The token will expire on ${expectedDate}`;
    assert.dom('[data-test-expiry-description]').hasText(expectedDescription);

    await fillIn('[data-test-name]', 'token-name');

    await select('[data-test-expiry]', 'none');
    assert.dom('[data-test-expiry-description]').hasText('The token will never expire');

    await select('[data-test-expiry]', '30');
    expiryDate = new Date('2017-12-20T00:00:00');
    expectedDate = expiryDate.toLocaleDateString(undefined, { dateStyle: 'long' });
    expectedDescription = `The token will expire on ${expectedDate}`;
    assert.dom('[data-test-expiry-description]').hasText(expectedDescription);

    await click('[data-test-scope="publish-update"]');
    await click('[data-test-generate]');

    let token = this.db.apiToken.findFirst({ where: { name: { equals: 'token-name' } } });
    assert.ok(Boolean(token), 'API token has been created in the backend database');
    assert.strictEqual(token.name, 'token-name');
    assert.strictEqual(token.expiredAt.slice(0, 10), '2017-12-20');
    assert.strictEqual(token.crateScopes, null);
    assert.deepEqual(token.endpointScopes, ['publish-update']);

    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token="1"] [data-test-name]').hasText('token-name');
    assert.dom('[data-test-api-token="1"] [data-test-token]').hasText(token.token);
    assert.dom('[data-test-api-token="1"] [data-test-endpoint-scopes]').hasText('Scopes: publish-update');
    assert.dom('[data-test-api-token="1"] [data-test-crate-scopes]').doesNotExist();
    assert.dom('[data-test-api-token="1"] [data-test-expired-at]').hasText('Expires in about 1 month');
  });

  test('token expiry with custom date', async function (assert) {
    prepare(this);

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await fillIn('[data-test-name]', 'token-name');
    await select('[data-test-expiry]', 'none');
    assert.dom('[data-test-expiry-description]').hasText('The token will never expire');
    await select('[data-test-expiry]', 'custom');
    assert.dom('[data-test-expiry-description]').doesNotExist();

    await click('[data-test-scope="publish-update"]');
    await click('[data-test-generate]');
    assert.dom('[data-test-expiry-date]').hasAria('invalid', 'true');

    await fillIn('[data-test-expiry-date]', '2024-05-04');
    assert.dom('[data-test-expiry-description]').doesNotExist();

    await click('[data-test-generate]');

    let token = this.db.apiToken.findFirst({ where: { name: { equals: 'token-name' } } });
    assert.ok(Boolean(token), 'API token has been created in the backend database');
    assert.strictEqual(token.name, 'token-name');
    assert.strictEqual(token.expiredAt.slice(0, 10), '2024-05-04');
    assert.strictEqual(token.crateScopes, null);
    assert.deepEqual(token.endpointScopes, ['publish-update']);

    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token="1"] [data-test-name]').hasText('token-name');
    assert.dom('[data-test-api-token="1"] [data-test-token]').hasText(token.token);
    assert.dom('[data-test-api-token="1"] [data-test-endpoint-scopes]').hasText('Scopes: publish-update');
    assert.dom('[data-test-api-token="1"] [data-test-crate-scopes]').doesNotExist();
    assert.dom('[data-test-api-token="1"] [data-test-expired-at]').hasText('Expires in over 6 years');
  });

  test('loading and error state', async function (assert) {
    prepare(this);

    let deferred = defer();
    this.worker.use(http.put('/api/v1/me/tokens', () => deferred.promise));

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await fillIn('[data-test-name]', 'token-name');
    await click('[data-test-scope="publish-update"]');
    let clickPromise = click('[data-test-generate]');
    await waitFor('[data-test-generate] [data-test-spinner]');
    assert.dom('[data-test-name]').isDisabled();
    assert.dom('[data-test-generate]').isDisabled();

    deferred.resolve(HttpResponse.json({}, { status: 500 }));
    await clickPromise;

    let message = 'An error has occurred while generating your API token. Please try again later!';
    assert.dom('[data-test-notification-message="error"]').hasText(message);
    assert.dom('[data-test-name]').isEnabled();
    assert.dom('[data-test-generate]').isEnabled();
  });

  test('cancel button navigates back to the token list', async function (assert) {
    prepare(this);

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await click('[data-test-cancel]');
    assert.strictEqual(currentURL(), '/settings/tokens');
  });

  test('empty name shows an error', async function (assert) {
    prepare(this);

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await click('[data-test-scope="publish-update"]');
    await click('[data-test-generate]');
    assert.strictEqual(currentURL(), '/settings/tokens/new');
    assert.dom('[data-test-name]').hasAria('invalid', 'true');
    assert.dom('[data-test-name-group] [data-test-error]').exists();
    assert.dom('[data-test-scopes-group] [data-test-error]').doesNotExist();
  });

  test('no scopes selected shows an error', async function (assert) {
    prepare(this);

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await fillIn('[data-test-name]', 'token-name');
    await click('[data-test-generate]');
    assert.strictEqual(currentURL(), '/settings/tokens/new');
    assert.dom('[data-test-name-group] [data-test-error]').doesNotExist();
    assert.dom('[data-test-scopes-group] [data-test-error]').exists();
  });

  test('prefill with the exist token', async function (assert) {
    let { user } = prepare(this);

    let token = this.db.apiToken.create({
      user,
      name: 'foo',
      createdAt: '2017-08-01T12:34:56',
      lastUsedAt: '2017-11-02T01:45:14',
      endpointScopes: ['publish-update'],
    });

    await visit(`/settings/tokens/new?from=${token.id}`);
    assert.strictEqual(currentURL(), `/settings/tokens/new?from=${token.id}`);
    assert.dom('[data-test-crates-unrestricted]').exists();
    assert.dom('[data-test-crate-pattern]').doesNotExist();

    await click('[data-test-add-crate-pattern]');
    assert.dom('[data-test-crates-unrestricted]').doesNotExist();
    assert.dom('[data-test-crate-pattern]').exists({ count: 1 });
    await fillIn('[data-test-crate-pattern="0"] input', 'serde');
    assert.dom('[data-test-crate-pattern="0"] [data-test-description]').hasText('Matches only the serde crate');
    await click('[data-test-generate]');
    assert.strictEqual(currentURL(), '/settings/tokens');

    let tokens = this.db.apiToken.findMany({ where: { name: { equals: 'foo' } } });
    assert.strictEqual(tokens.length, 2, 'New API token has been created in the backend database');

    // It should reset the token ID query parameter.
    await click('[data-test-new-token-button]');
    assert.strictEqual(currentURL(), '/settings/tokens/new');
  });

  test('prefilled: crate scoped can be added', async function (assert) {
    let { user } = prepare(this);

    let token = this.db.apiToken.create({
      user,
      name: 'serde',
      crateScopes: ['serde', 'serde-*'],
      endpointScopes: ['publish-update'],
    });

    await visit(`/settings/tokens/new?from=${token.id}`);
    assert.strictEqual(currentURL(), `/settings/tokens/new?from=${token.id}`);
    assert.dom('[data-test-crate-pattern]').exists({ count: 2 });

    await click('[data-test-add-crate-pattern]');
    assert.dom('[data-test-crate-pattern]').exists({ count: 3 });
    await fillIn('[data-test-crate-pattern="2"] input', 'serde2');
    await click('[data-test-generate]');
    assert.strictEqual(currentURL(), '/settings/tokens');
  });

  test('token not found', async function (assert) {
    prepare(this);

    await visit('/settings/tokens/new?from=1');
    assert.strictEqual(currentURL(), '/settings/tokens/new?from=1');
    assert.dom('[data-test-title]').hasText('Token not found');
  });
});
