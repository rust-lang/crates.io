import { click, currentURL, visit, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupApplicationTest } from 'cargo/tests/helpers';

module('Acceptance | Login', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);

  test('successful login', async function (assert) {
    let deferred = defer();

    window.open = (url, target, features) => {
      assert.equal(url, '');
      assert.equal(target, '_blank');
      assert.equal(features, 'width=1000,height=450,toolbar=0,scrollbars=1,status=1,resizable=1,location=1,menuBar=0');
      deferred.resolve();
      return { document: { write() {}, close() {} }, close() {} };
    };

    this.server.get('/api/private/session/begin', { url: 'url-to-github-including-state-secret' });

    this.server.get('/api/private/session/authorize', (schema, request) => {
      assert.deepEqual(request.queryParams, {
        code: '901dd10e07c7e9fa1cd5',
        state: 'fYcUY3FMdUUz00FC7vLT7A',
      });

      let user = this.server.create('user');
      this.server.create('mirage-session', { user });
      return { ok: true };
    });

    this.server.get('/api/v1/me', () => ({
      user: {
        id: 42,
        login: 'johnnydee',
        name: 'John Doe',
        email: 'john@doe.name',
        avatar: 'https://avatars2.githubusercontent.com/u/12345?v=4',
        url: 'https://github.com/johnnydee',
      },
      owned_crates: [],
    }));

    await visit('/');
    assert.equal(currentURL(), '/');

    await click('[data-test-login-button]');
    assert.equal(currentURL(), '/');

    // wait for `window.open()` to be called
    await deferred.promise;

    // simulate the response from the `github-authorize` route
    let message = { code: '901dd10e07c7e9fa1cd5', state: 'fYcUY3FMdUUz00FC7vLT7A' };
    window.postMessage(message, window.location.origin);

    // wait for the user menu to show up after the successful login
    await waitFor('[data-test-user-menu]');

    assert.dom('[data-test-user-menu] [data-test-toggle]').hasText('John Doe');
  });

  test('failed login', async function (assert) {
    let deferred = defer();

    window.open = () => {
      deferred.resolve();
      return { document: { write() {}, close() {} }, close() {} };
    };

    this.server.get('/api/private/session/begin', { url: 'url-to-github-including-state-secret' });

    let payload = { errors: [{ detail: 'Forbidden' }] };
    this.server.get('/api/private/session/authorize', payload, 403);

    await visit('/');
    assert.equal(currentURL(), '/');

    await click('[data-test-login-button]');
    assert.equal(currentURL(), '/');

    // wait for `window.open()` to be called
    await deferred.promise;

    // simulate the response from the `github-authorize` route
    let message = { code: '901dd10e07c7e9fa1cd5', state: 'fYcUY3FMdUUz00FC7vLT7A' };
    window.postMessage(message, window.location.origin);

    // wait for the error message to show up after the failed login
    await waitFor(`[data-test-notification-message]`);

    assert.dom('[data-test-notification-message]').hasText('Failed to log in: Forbidden');
  });
});
