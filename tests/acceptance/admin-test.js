import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Admin', function (hooks) {
  setupApplicationTest(hooks);

  test('shows "page requires admin authentication" error when not logged in', async function (assert) {
    await visit('/admin');
    assert.equal(currentURL(), '/admin');
    assert.dom('[data-test-title]').hasText('This page requires admin authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('shows "page requires admin authentication" error when logged in but not as an admin', async function (assert) {
    let user = this.server.create('user', {
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      admin: false,
    });

    this.authenticateAs(user);

    await visit('/admin');
    assert.equal(currentURL(), '/admin');
    assert.dom('[data-test-title]').hasText('This page requires admin authentication');
    assert.dom('[data-test-login]').doesNotExist();
  });

  test('shows admin actions when logged in as an admin', async function (assert) {
    let user = this.server.create('user', {
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      admin: true,
    });

    this.authenticateAs(user);

    await visit('/admin');
    // Rate limits is the default action.
    assert.equal(currentURL(), '/admin/rate-limits');
    assert.dom('[data-test-heading]').hasText('Admin Actions');
    assert.dom('[data-test-login]').doesNotExist();
  });

  module('Rate limits', function () {
    test('shows "page requires admin authentication" error when not logged in', async function (assert) {
      await visit('/admin/rate-limits');
      assert.equal(currentURL(), '/admin/rate-limits');
      assert.dom('[data-test-title]').hasText('This page requires admin authentication');
      assert.dom('[data-test-login]').exists();
    });

    test('shows "page requires admin authentication" error when logged in but not as an admin', async function (assert) {
      let user = this.server.create('user', {
        login: 'johnnydee',
        name: 'John Doe',
        email: 'john@doe.com',
        avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
        admin: false,
      });

      this.authenticateAs(user);

      await visit('/admin/rate-limits');
      assert.equal(currentURL(), '/admin/rate-limits');
      assert.dom('[data-test-title]').hasText('This page requires admin authentication');
      assert.dom('[data-test-login]').doesNotExist();
    });
  });

  test('shows rate limit actions when logged in as an admin', async function (assert) {
    let user = this.server.create('user', {
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      admin: true,
    });

    this.authenticateAs(user);

    await visit('/admin/rate-limits');
    assert.equal(currentURL(), '/admin/rate-limits');
    assert.dom('[data-test-heading]').hasText('Admin Actions');
    assert.dom('[data-test-login]').doesNotExist();
  });
});
