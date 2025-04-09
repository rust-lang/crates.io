import { click, currentURL, visit, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

module('Acceptance | Login', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);

  test('successful login', async function (assert) {
    let { db } = this;
    let deferred = defer();

    window.open = (url, target, features) => {
      assert.strictEqual(url, '');
      assert.strictEqual(target, '_blank');
      assert.strictEqual(
        features,
        'width=1000,height=450,toolbar=0,scrollbars=1,status=1,resizable=1,location=1,menuBar=0',
      );
      deferred.resolve();
      return { document: { write() {}, close() {} }, close() {} };
    };

    this.worker.use(
      http.get('/api/private/session/begin', () => HttpResponse.json({ url: 'url-to-github-including-state-secret' })),
      http.get('/api/private/session/authorize', ({ request }) => {
        let url = new URL(request.url);
        assert.deepEqual([...url.searchParams.keys()], ['code', 'state']);
        assert.strictEqual(url.searchParams.get('code'), '901dd10e07c7e9fa1cd5');
        assert.strictEqual(url.searchParams.get('state'), 'fYcUY3FMdUUz00FC7vLT7A');

        let user = db.user.create();
        db.mswSession.create({ user });
        return HttpResponse.json({ ok: true });
      }),
      http.get('/api/v1/me', () =>
        HttpResponse.json({
          user: {
            id: 42,
            login: 'johnnydee',
            name: 'John Doe',
            email: 'john@doe.name',
            avatar: 'https://avatars2.githubusercontent.com/u/12345?v=4',
            url: 'https://github.com/johnnydee',
          },
          owned_crates: [],
        }),
      ),
    );

    await visit('/');
    assert.strictEqual(currentURL(), '/');

    await click('[data-test-login-button]');
    assert.strictEqual(currentURL(), '/');

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

    this.worker.use(
      http.get('/api/private/session/begin', () => HttpResponse.json({ url: 'url-to-github-including-state-secret' })),
      http.get('/api/private/session/authorize', () =>
        HttpResponse.json({ errors: [{ detail: 'Forbidden' }] }, { status: 403 }),
      ),
    );

    await visit('/');
    assert.strictEqual(currentURL(), '/');

    await click('[data-test-login-button]');
    assert.strictEqual(currentURL(), '/');

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
