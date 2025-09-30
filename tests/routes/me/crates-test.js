import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../helpers/visit-ignoring-abort';

module('Route | me/crates', function (hooks) {
  setupApplicationTest(hooks);

  test('redirects to user profile page', async function (assert) {
    let user = this.db.user.create({ login: 'johnnydee' });
    this.authenticateAs(user);

    await visit('/me/crates?page=2&sort=downloads');
    assert.strictEqual(currentURL(), '/users/johnnydee?page=2&sort=downloads');
  });
});
