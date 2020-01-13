import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { visit, currentURL, click, waitFor } from '@ember/test-helpers';
import { defer } from 'rsvp';
import window, { setupWindowMock } from 'ember-window-mock';

import flashStyles from 'cargo/components/flash-message.module.scss';
import setupMirage from '../helpers/setup-mirage';

module('Acceptance | Login', function(hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);
  setupMirage(hooks);

  test('successful login', async function(assert) {
    let deferred = defer();
    let fakeWindow = { closed: false };
    window.open = (url, target, features) => {
      assert.equal(url, '/github_login');
      assert.equal(target, 'Authorization');
      assert.equal(features, 'width=1000,height=450,toolbar=0,scrollbars=1,status=1,resizable=1,location=1,menuBar=0');
      deferred.resolve();
      return fakeWindow;
    };

    await visit('/');
    assert.equal(currentURL(), '/');

    await click('[data-test-login-link]');
    assert.equal(currentURL(), '/');

    // wait for `window.open()` to be called
    await deferred.promise;

    // simulate the response from the `github-authorize` route
    window.github_response = JSON.stringify({
      ok: true,
      data: {
        user: {
          id: 42,
          login: 'johnnydee',
          name: 'John Doe',
          email: 'john@doe.name',
          avatar: 'https://avatars2.githubusercontent.com/u/12345?v=4',
          url: 'https://github.com/johnnydee',
        },
      },
    });

    // simulate that the window has been closed by the `github-authorize` route
    fakeWindow.closed = true;

    // wait for the user menu to show up after the successful login
    await waitFor('[data-test-user-menu]');

    assert.dom('[data-test-user-menu] [data-test-toggle]').hasText('John Doe');
  });

  test('failed login', async function(assert) {
    let deferred = defer();
    let fakeWindow = { closed: false };
    window.open = () => {
      deferred.resolve();
      return fakeWindow;
    };

    await visit('/');
    assert.equal(currentURL(), '/');

    await click('[data-test-login-link]');
    assert.equal(currentURL(), '/');

    // wait for `window.open()` to be called
    await deferred.promise;

    // simulate the response from the `github-authorize` route
    window.github_response = JSON.stringify({
      ok: false,
      data: {
        errors: [{ detail: 'Forbidden' }],
      },
    });

    // simulate that the window has been closed by the `github-authorize` route
    fakeWindow.closed = true;

    // wait for the error message to show up after the failed login
    await waitFor(`[data-test-flash-message].${flashStyles.shown}`);

    assert.dom('[data-test-flash-message]').hasText('Failed to log in: Forbidden');
  });
});
