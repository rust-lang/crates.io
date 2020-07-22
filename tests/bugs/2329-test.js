import { currentURL, click, waitFor } from '@ember/test-helpers';
import { setupApplicationTest } from 'ember-qunit';
import { module, test } from 'qunit';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import setupMirage from '../helpers/setup-mirage';
import { visit } from '../helpers/visit-ignoring-abort';

module('Bug #2329', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);
  setupMirage(hooks);

  test('is fixed', async function (assert) {
    let user = {
      id: 42,
      login: 'johnnydee',
      email_verified: true,
      email_verification_sent: true,
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      url: 'https://github.com/johnnydee',
    };

    this.server.get('/api/v1/me', {
      user,
      owned_crates: [
        { id: 123, name: 'foo-bar', email_notifications: true },
        { id: 56456, name: 'barrrrr', email_notifications: false },
      ],
    });

    this.server.get('/api/v1/me/tokens', { api_tokens: [] });

    let fakeWindow = { closed: false };
    window.open = () => fakeWindow;

    // 1. Sign out.
    window.localStorage.removeItem('isLoggedIn');

    // 2. Open the network debug panel.
    // ...

    // 3. Refresh the page.
    await visit('/');

    // 4. Click the "Log in with GitHub" link.
    await click('[data-test-login-link]');

    // 5. Complete the authentication workflow if necessary.

    // simulate the response from the `github-authorize` route
    window.github_response = JSON.stringify({ ok: true, data: { user } });

    // simulate that the window has been closed by the `github-authorize` route
    fakeWindow.closed = true;

    // wait for the user menu to show up after the successful login
    await waitFor('[data-test-user-menu]');

    // 6. Use the menu to navigate to "Account Settings".
    await click('[data-test-user-menu]');
    await click('[data-test-me-link]');
    await visit('/me');
    assert.equal(currentURL(), '/me');

    // 7. Observe that there are no crates listed under "Email Notification Preferences".
    // Observe that no request was sent to the /api/v1/me endpoint to obtain this data.

    // here we divert from the reproduction instructions, since this is the
    // issue that we want to fix
    assert.dom('[data-test-owned-crate]').exists({ count: 2 });
  });
});
