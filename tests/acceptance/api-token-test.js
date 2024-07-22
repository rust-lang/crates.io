import { click, currentURL, fillIn, findAll } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import { Response } from 'miragejs';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | api-tokens', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let user = context.server.create('user', {
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });

    context.server.create('api-token', {
      user,
      name: 'foo',
      createdAt: '2017-08-01T12:34:56',
      lastUsedAt: '2017-11-02T01:45:14',
    });

    context.server.create('api-token', {
      user,
      name: 'BAR',
      createdAt: '2017-11-19T17:59:22',
      lastUsedAt: null,
      expiredAt: '2017-12-19T17:59:22',
    });

    context.server.create('api-token', {
      user,
      name: 'recently expired',
      createdAt: '2017-08-01T12:34:56',
      lastUsedAt: '2017-11-02T01:45:14',
      expiredAt: '2017-11-19T17:59:22',
    });

    context.authenticateAs(user);
  }

  test('/me is showing the list of active API tokens', async function (assert) {
    prepare(this);

    await visit('/settings/tokens');
    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token]').exists({ count: 3 });

    let [row1, row2, row3] = findAll('[data-test-api-token]');
    assert.dom('[data-test-name]', row1).hasText('BAR');
    assert.dom('[data-test-created-at]', row1).hasText('Created about 18 hours ago');
    assert.dom('[data-test-last-used-at]', row1).hasText('Never used');
    assert.dom('[data-test-expired-at]', row1).hasText('Expires in 29 days');
    assert.dom('[data-test-save-token-button]', row1).doesNotExist();
    assert.dom('[data-test-revoke-token-button]', row1).exists();
    assert.dom('[data-test-saving-spinner]', row1).doesNotExist();
    assert.dom('[data-test-error]', row1).doesNotExist();
    assert.dom('[data-test-token]', row1).doesNotExist();

    assert.dom('[data-test-name]', row2).hasText('foo');
    assert.dom('[data-test-created-at]', row2).hasText('Created 4 months ago');
    assert.dom('[data-test-last-used-at]', row2).hasText('Last used 18 days ago');
    assert.dom('[data-test-expired-at]', row2).doesNotExist();
    assert.dom('[data-test-save-token-button]', row2).doesNotExist();
    assert.dom('[data-test-revoke-token-button]', row2).exists();
    assert.dom('[data-test-saving-spinner]', row2).doesNotExist();
    assert.dom('[data-test-error]', row2).doesNotExist();
    assert.dom('[data-test-token]', row2).doesNotExist();

    assert.dom('[data-test-name]', row3).hasText('recently expired');
    assert.dom('[data-test-created-at]', row3).hasText('Created 4 months ago');
    assert.dom('[data-test-last-used-at]', row3).hasText('Last used 18 days ago');
    assert.dom('[data-test-expired-at]', row3).hasText('Expired about 18 hours ago');
    assert.dom('[data-test-save-token-button]', row3).doesNotExist();
    assert.dom('[data-test-revoke-token-button]', row3).doesNotExist();
    assert.dom('[data-test-saving-spinner]', row3).doesNotExist();
    assert.dom('[data-test-error]', row3).doesNotExist();
    assert.dom('[data-test-token]', row3).doesNotExist();
  });

  test('API tokens can be revoked', async function (assert) {
    prepare(this);

    await visit('/settings/tokens');
    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token]').exists({ count: 3 });

    await click('[data-test-api-token="1"] [data-test-revoke-token-button]');
    assert.strictEqual(
      this.server.schema.apiTokens.all().length,
      2,
      'API token has been deleted from the backend database',
    );

    assert.dom('[data-test-api-token]').exists({ count: 2 });
    assert.dom('[data-test-api-token="2"]').exists();
    assert.dom('[data-test-error]').doesNotExist();
  });

  test('API tokens can be regenerated', async function (assert) {
    prepare(this);

    await visit('/settings/tokens');
    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token]').exists({ count: 3 });

    await click('[data-test-api-token="1"] [data-test-regenerate-token-button]');
    assert.strictEqual(currentURL(), '/settings/tokens/new?from=1');
  });

  test('failed API tokens revocation shows an error', async function (assert) {
    prepare(this);

    this.server.delete('/api/v1/me/tokens/:id', function () {
      return new Response(500, {}, {});
    });

    await visit('/settings/tokens');
    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token]').exists({ count: 3 });

    await click('[data-test-api-token="1"] [data-test-revoke-token-button]');
    assert.dom('[data-test-api-token]').exists({ count: 3 });
    assert.dom('[data-test-api-token="2"]').exists();
    assert.dom('[data-test-api-token="1"]').exists();
    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('An unknown error occurred while revoking this token');
  });

  test('new API tokens can be created', async function (assert) {
    prepare(this);

    await visit('/settings/tokens');
    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token]').exists({ count: 3 });

    await click('[data-test-new-token-button]');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await fillIn('[data-test-name]', 'the new token');
    await click('[data-test-scope="publish-update"]');
    await percySnapshot(assert);

    await click('[data-test-generate]');

    let token = this.server.schema.apiTokens.findBy({ name: 'the new token' });
    assert.ok(Boolean(token), 'API token has been created in the backend database');

    assert.dom('[data-test-api-token="4"] [data-test-name]').hasText('the new token');
    assert.dom('[data-test-api-token="4"] [data-test-save-token-button]').doesNotExist();
    assert.dom('[data-test-api-token="4"] [data-test-revoke-token-button]').exists();
    assert.dom('[data-test-api-token="4"] [data-test-saving-spinner]').doesNotExist();
    assert.dom('[data-test-api-token="4"] [data-test-error]').doesNotExist();
    assert.dom('[data-test-token]').hasText(token.token);
  });

  test('API tokens are only visible in plaintext until the page is left', async function (assert) {
    prepare(this);

    await visit('/settings/tokens');
    await click('[data-test-new-token-button]');
    await fillIn('[data-test-name]', 'the new token');
    await click('[data-test-scope="publish-update"]');
    await click('[data-test-generate]');

    let token = this.server.schema.apiTokens.findBy({ name: 'the new token' });
    assert.dom('[data-test-token]').hasText(token.token);

    // leave the API tokens page
    await visit('/settings');

    // and visit it again
    await visit('/settings/tokens');
    assert.dom('[data-test-token]').doesNotExist();
  });

  test('navigating away while creating a token does not keep it in the list', async function (assert) {
    prepare(this);

    await visit('/settings/tokens');
    assert.dom('[data-test-api-token]').exists({ count: 3 });

    await click('[data-test-new-token-button]');

    await visit('/settings/profile');

    await visit('/settings/tokens');
    assert.dom('[data-test-api-token]').exists({ count: 3 });
  });
});
