import { click, currentURL, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupApplicationTest } from 'cargo/tests/helpers';

module('Acceptance | Logout', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);

  test('successful logout', async function (assert) {
    let user = this.server.create('user', { name: 'John Doe' });
    this.authenticateAs(user);

    await visit('/crates');
    assert.equal(currentURL(), '/crates');
    assert.dom('[data-test-user-menu] [data-test-toggle]').hasText('John Doe');

    await click('[data-test-user-menu] [data-test-toggle]');
    await click('[data-test-user-menu] [data-test-logout-button]');

    assert.equal(window.location.pathname, '/');
  });
});
