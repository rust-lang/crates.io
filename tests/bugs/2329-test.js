import { click, currentURL, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Bug #2329', function (hooks) {
  setupApplicationTest(hooks, { msw: true });
  setupWindowMock(hooks);

  test('is fixed', async function (assert) {
    let { db } = this;

    let user = this.db.user.create();

    let foobar = this.db.crate.create({ name: 'foo-bar' });
    this.db.crateOwnership.create({ crate: foobar, user, emailNotifications: true });
    this.db.version.create({ crate: foobar });

    let bar = this.db.crate.create({ name: 'barrrrr' });
    this.db.crateOwnership.create({ crate: bar, user, emailNotifications: false });
    this.db.version.create({ crate: bar });

    this.worker.use(
      http.get('/api/private/session/begin', () => HttpResponse.json({ url: 'url-to-github-including-state-secret' })),
      http.get('/api/private/session/authorize', () => {
        db.mswSession.create({ user });
        return HttpResponse.json({ ok: true });
      }),
    );

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
    assert.strictEqual(currentURL(), '/settings/email-notifications');

    // 7. Observe that there are no crates listed under "Email Notification Preferences".
    // Observe that no request was sent to the /api/v1/me endpoint to obtain this data.

    // here we divert from the reproduction instructions, since this is the
    // issue that we want to fix
    assert.dom('[data-test-owned-crate]').exists({ count: 2 });
  });
});
