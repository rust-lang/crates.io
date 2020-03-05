import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { currentURL, findAll } from '@ember/test-helpers';
import window, { setupWindowMock } from 'ember-window-mock';

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
});
