import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { currentURL, findAll, click, fillIn } from '@ember/test-helpers';
import window, { setupWindowMock } from 'ember-window-mock';
import { Response } from 'ember-cli-mirage';
import { percySnapshot } from 'ember-percy';

import setupMirage from '../helpers/setup-mirage';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | api-tokens', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);
  setupMirage(hooks);

  function prepare(context) {
    window.localStorage.setItem('isLoggedIn', '1');

    context.server.get('/api/v1/me', {
      user: {
        id: 42,
        login: 'johnnydee',
        email_verified: true,
        email_verification_sent: true,
        name: 'John Doe',
        email: 'john@doe.com',
        avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
        url: 'https://github.com/johnnydee',
      },
      owned_crates: [],
    });

    context.server.get('/api/v1/me/tokens', {
      api_tokens: [
        { id: 2, name: 'BAR', created_at: new Date('2017-11-19T17:59:22').toISOString(), last_used_at: null },
        {
          id: 1,
          name: 'foo',
          created_at: new Date('2017-08-01T12:34:56').toISOString(),
          last_used_at: new Date('2017-11-02T01:45:14').toISOString(),
        },
      ],
    });
  }

  test('/me is showing the list of active API tokens', async function (assert) {
    prepare(this);

    await visit('/me');
    assert.equal(currentURL(), '/me');
    assert.dom('[data-test-api-token]').exists({ count: 2 });

    let [row1, row2] = findAll('[data-test-api-token]');
    assert.dom('[data-test-name]', row1).hasText('BAR');
    assert.dom('[data-test-created-at]', row1).hasText('Created 18 hours ago');
    assert.dom('[data-test-last-used-at]', row1).hasText('Never used');
    assert.dom('[data-test-save-token-button]', row1).doesNotExist();
    assert.dom('[data-test-revoke-token-button]', row1).exists();
    assert.dom('[data-test-saving-spinner]', row1).doesNotExist();
    assert.dom('[data-test-error]', row1).doesNotExist();
    assert.dom('[data-test-token]', row1).doesNotExist();

    assert.dom('[data-test-name]', row2).hasText('foo');
    assert.dom('[data-test-created-at]', row2).hasText('Created 4 months ago');
    assert.dom('[data-test-last-used-at]', row2).hasText('Last used 18 days ago');
    assert.dom('[data-test-save-token-button]', row2).doesNotExist();
    assert.dom('[data-test-revoke-token-button]', row2).exists();
    assert.dom('[data-test-saving-spinner]', row2).doesNotExist();
    assert.dom('[data-test-error]', row2).doesNotExist();
    assert.dom('[data-test-token]', row2).doesNotExist();
  });

  test('API tokens can be revoked', async function (assert) {
    prepare(this);

    this.server.delete('/api/v1/me/tokens/:id', function (schema, request) {
      assert.step(`delete id:${request.params.id}`);
      return {};
    });

    await visit('/me');
    assert.equal(currentURL(), '/me');
    assert.dom('[data-test-api-token]').exists({ count: 2 });

    await click('[data-test-api-token="1"] [data-test-revoke-token-button]');
    assert.verifySteps(['delete id:1']);

    assert.dom('[data-test-api-token]').exists({ count: 1 });
    assert.dom('[data-test-api-token="2"]').exists();
    assert.dom('[data-test-error]').doesNotExist();
  });

  test('failed API tokens revocation shows an error', async function (assert) {
    prepare(this);

    this.server.delete('/api/v1/me/tokens/:id', function () {
      return new Response(500, {}, {});
    });

    await visit('/me');
    assert.equal(currentURL(), '/me');
    assert.dom('[data-test-api-token]').exists({ count: 2 });

    await click('[data-test-api-token="1"] [data-test-revoke-token-button]');
    assert.dom('[data-test-api-token]').exists({ count: 2 });
    assert.dom('[data-test-api-token="2"]').exists();
    assert.dom('[data-test-api-token="1"]').exists();
    assert.dom('[data-test-error]').includesText('An error occurred while revoking this token');
  });

  test('new API tokens can be created', async function (assert) {
    prepare(this);

    this.server.put('/api/v1/me/tokens', function (schema, request) {
      assert.step('put');

      let { api_token } = JSON.parse(request.requestBody);

      return {
        api_token: {
          id: 5,
          name: api_token.name,
          token: 'zuz6nYcXJOzPDvnA9vucNwccG0lFSGbh',
          revoked: false,
          created_at: api_token.created_at,
          last_used_at: api_token.last_used_at,
        },
      };
    });

    await visit('/me');
    assert.equal(currentURL(), '/me');
    assert.dom('[data-test-api-token]').exists({ count: 2 });
    assert.dom('[data-test-focused-input]').doesNotExist();
    assert.dom('[data-test-save-token-button]').doesNotExist();

    await click('[data-test-new-token-button]');
    assert.dom('[data-test-new-token-button]').isDisabled();
    assert.dom('[data-test-focused-input]').exists();
    assert.dom('[data-test-save-token-button]').exists();

    await fillIn('[data-test-focused-input]', 'the new token');
    percySnapshot(assert);

    await click('[data-test-save-token-button]');
    assert.verifySteps(['put']);
    assert.dom('[data-test-focused-input]').doesNotExist();
    assert.dom('[data-test-save-token-button]').doesNotExist();

    assert.dom('[data-test-api-token="5"] [data-test-name]').hasText('the new token');
    assert.dom('[data-test-api-token="5"] [data-test-save-token-button]').doesNotExist();
    assert.dom('[data-test-api-token="5"] [data-test-revoke-token-button]').exists();
    assert.dom('[data-test-api-token="5"] [data-test-saving-spinner]').doesNotExist();
    assert.dom('[data-test-api-token="5"] [data-test-error]').doesNotExist();
    assert.dom('[data-test-token]').includesText('cargo login zuz6nYcXJOzPDvnA9vucNwccG0lFSGbh');
  });
});
