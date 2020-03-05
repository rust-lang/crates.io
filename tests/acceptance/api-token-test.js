import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { currentURL, findAll, click } from '@ember/test-helpers';
import window, { setupWindowMock } from 'ember-window-mock';
import { Response } from 'ember-cli-mirage';

import setupMirage from '../helpers/setup-mirage';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | api-tokens', function(hooks) {
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
        { id: 2, name: 'BAR', created_at: '2017-11-19T17:59:22Z', last_used_at: null },
        { id: 1, name: 'foo', created_at: '2017-08-01T12:34:56Z', last_used_at: '2017-11-02T01:45:14Z' },
      ],
    });
  }

  test('/me is showing the list of active API tokens', async function(assert) {
    prepare(this);

    await visit('/me');
    assert.equal(currentURL(), '/me');
    assert.dom('[data-test-api-token]').exists({ count: 2 });

    let [row1, row2] = findAll('[data-test-api-token]');
    assert.dom('[data-test-name]', row1).hasText('BAR');
    assert.dom('[data-test-created-at]', row1).hasText('Created 17 hours ago');
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

  test('API tokens can be revoked', async function(assert) {
    prepare(this);

    this.server.delete('/api/v1/me/tokens/:id', function(schema, request) {
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

  test('failed API tokens revocation shows an error', async function(assert) {
    prepare(this);

    this.server.delete('/api/v1/me/tokens/:id', function() {
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
});
