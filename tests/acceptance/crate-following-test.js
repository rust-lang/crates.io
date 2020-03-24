import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { visit, waitFor, settled, click } from '@ember/test-helpers';
import window, { setupWindowMock } from 'ember-window-mock';
import { defer } from 'rsvp';

import setupMirage from '../helpers/setup-mirage';

module('Acceptance | Crate following', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);
  setupMirage(hooks);

  function prepare(context, { loggedIn = true } = {}) {
    let server = context.server;

    let crate = server.create('crate', { name: 'nanomsg' });
    server.create('version', { crate, num: '0.6.0' });

    if (loggedIn) {
      server.get('/api/v1/me', {
        user: {
          id: 42,
          login: 'johnnydee',
          email_verified: true,
          email_verification_sent: true,
          name: 'John Doe',
          email: 'john@doe.com',
          avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
          url: 'https://github.com/johnnydee',
        },
        owned_crates: [],
      });

      window.localStorage.setItem('isLoggedIn', '1');
    }
  }

  test("unauthenticated users don't see the follow button", async function (assert) {
    prepare(this, { loggedIn: false });

    await visit('/crates/nanomsg');
    assert.dom('[data-test-follow-button]').doesNotExist();
  });

  test('authenticated users see a loading spinner and can follow/unfollow crates', async function (assert) {
    prepare(this);

    let followingDeferred = defer();
    this.server.get('/api/v1/crates/:crate_id/following', followingDeferred.promise);

    visit('/crates/nanomsg');
    await waitFor('[data-test-follow-button] [data-test-spinner]');
    assert.dom('[data-test-follow-button]').hasText('');
    assert.dom('[data-test-follow-button] [data-test-spinner]').exists();

    followingDeferred.resolve({ following: false });
    await settled();
    assert.dom('[data-test-follow-button]').hasText('Follow');
    assert.dom('[data-test-follow-button] [data-test-spinner]').doesNotExist();

    let followDeferred = defer();
    this.server.put('/api/v1/crates/:crate_id/follow', followDeferred.promise);

    click('[data-test-follow-button]');
    await waitFor('[data-test-follow-button] [data-test-spinner]');
    assert.dom('[data-test-follow-button]').hasText('');
    assert.dom('[data-test-follow-button] [data-test-spinner]').exists();

    followDeferred.resolve({ ok: true });
    await settled();
    assert.dom('[data-test-follow-button]').hasText('Unfollow');
    assert.dom('[data-test-follow-button] [data-test-spinner]').doesNotExist();

    let unfollowDeferred = defer();
    this.server.delete('/api/v1/crates/:crate_id/follow', unfollowDeferred.promise);

    click('[data-test-follow-button]');
    await waitFor('[data-test-follow-button] [data-test-spinner]');
    assert.dom('[data-test-follow-button]').hasText('');
    assert.dom('[data-test-follow-button] [data-test-spinner]').exists();

    unfollowDeferred.resolve({ ok: true });
    await settled();
    assert.dom('[data-test-follow-button]').hasText('Follow');
    assert.dom('[data-test-follow-button] [data-test-spinner]').doesNotExist();
  });
});
