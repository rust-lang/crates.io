import { click, currentURL, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Bug #2329', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);

  test('is fixed', async function (assert) {
    let user = this.server.create('user');

    let foobar = this.server.create('crate', { name: 'foo-bar' });
    this.server.create('crate-ownership', { crate: foobar, user, emailNotifications: true });
    this.server.create('version', { crate: foobar });

    let bar = this.server.create('crate', { name: 'barrrrr' });
    this.server.create('crate-ownership', { crate: bar, user, emailNotifications: false });
    this.server.create('version', { crate: bar });

    this.server.get('/api/private/session/begin', { url: 'url-to-github-including-state-secret' });

    this.server.get('/api/private/session/authorize', () => {
      this.server.create('mirage-session', { user });
      return { ok: true };
    });

    let fakeWindow = { document: { write() {}, close() {} }, close() {} };
    window.open = () => fakeWindow;

    // 1. Sign out.
    window.localStorage.removeItem('isLoggedIn');

    // 2. Open the network debug panel.
    // ...

    // 3. Refresh the page.
    await visit('/');

    // 4. Click the "Log in with GitHub" link.
    await click('[data-test-login-button]');

    // 5. Complete the authentication workflow if necessary.

    // simulate the response from the `github-authorize` route
    window.postMessage({ code: 'foo', state: 'bar' }, window.location.origin);

    // wait for the user menu to show up after the successful login
    await waitFor('[data-test-user-menu]');

    // 6. Use the menu to navigate to "Account Settings".
    await click('[data-test-user-menu]');
    await click('[data-test-me-link]');
    await visit('/settings/email-notifications');
    assert.equal(currentURL(), '/settings/email-notifications');

    // 7. Observe that there are no crates listed under "Email Notification Preferences".
    // Observe that no request was sent to the /api/v1/me endpoint to obtain this data.

    // here we divert from the reproduction instructions, since this is the
    // issue that we want to fix
    assert.dom('[data-test-owned-crate]').exists({ count: 2 });
  });
});
