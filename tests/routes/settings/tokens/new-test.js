import { click, currentURL, fillIn, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import { Response } from 'miragejs';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../../../helpers/visit-ignoring-abort';

module('/settings/tokens/new', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let user = context.server.create('user', {
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });

    context.authenticateAs(user);
  }

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
    await click('[data-test-generate]');

    let token = this.server.schema.apiTokens.findBy({ name: 'token-name' });
    assert.ok(Boolean(token), 'API token has been created in the backend database');

    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token="1"] [data-test-name]').hasText('token-name');
    assert.dom('[data-test-api-token="1"] [data-test-token]').hasText(token.token);
  });

  test('loading and error state', async function (assert) {
    prepare(this);

    let deferred = defer();
    this.server.put('/api/v1/me/tokens', deferred.promise);

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await fillIn('[data-test-name]', 'token-name');
    let clickPromise = click('[data-test-generate]');
    await waitFor('[data-test-generate] [data-test-spinner]');
    assert.dom('[data-test-name]').isDisabled();
    assert.dom('[data-test-generate]').isDisabled();

    deferred.resolve(new Response(500));
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

    await click('[data-test-generate]');
    assert.strictEqual(currentURL(), '/settings/tokens/new');
    assert.dom('[data-test-name]').hasAria('invalid', 'true');
  });
});
