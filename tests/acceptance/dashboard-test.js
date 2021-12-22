import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Dashboard', function (hooks) {
  setupApplicationTest(hooks);

  test('shows "page requires authentication" error when not logged in', async function (assert) {
    await visit('/dashboard');
    assert.equal(currentURL(), '/dashboard');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('shows the dashboard when logged in', async function (assert) {
    let user = this.server.create('user', {
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });

    this.authenticateAs(user);

    {
      let crate = this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crate, num: '0.5.0' });
      this.server.create('version', { crate, num: '0.6.0' });
      this.server.create('version', { crate, num: '0.7.0' });
      this.server.create('version', { crate, num: '0.7.1' });
      this.server.create('version', { crate, num: '0.7.2' });
      this.server.create('version', { crate, num: '0.7.3' });
      this.server.create('version', { crate, num: '0.8.0' });
      this.server.create('version', { crate, num: '0.8.1' });
      this.server.create('version', { crate, num: '0.9.0' });
      this.server.create('version', { crate, num: '1.0.0' });
      this.server.create('version', { crate, num: '1.1.0' });
      user.followedCrates.add(crate);
    }

    {
      let crate = this.server.create('crate', { name: 'nanomsg' });
      this.server.create('crate-ownership', { crate, user });
      this.server.create('version', { crate, num: '0.1.0' });
      user.followedCrates.add(crate);
    }

    user.save();

    this.server.get(`/api/v1/users/${user.id}/stats`, { total_downloads: 3892 });

    await visit('/dashboard');
    assert.equal(currentURL(), '/dashboard');
    await percySnapshot(assert);
  });
});
