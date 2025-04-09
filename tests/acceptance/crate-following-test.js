import { click, settled, visit, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

module('Acceptance | Crate following', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context, { loggedIn = true, following = false } = {}) {
    let { db } = context;

    let crate = db.crate.create({ name: 'nanomsg' });
    db.version.create({ crate, num: '0.6.0' });

    if (loggedIn) {
      let followedCrates = following ? [crate] : [];
      let user = db.user.create({ followedCrates });
      context.authenticateAs(user);
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
    this.worker.use(http.get('/api/v1/crates/:crate_id/following', () => followingDeferred.promise));

    visit('/crates/nanomsg');
    await waitFor('[data-test-follow-button] [data-test-spinner]');
    assert.dom('[data-test-follow-button]').hasText('Loading…').isDisabled();
    assert.dom('[data-test-follow-button] [data-test-spinner]').exists();

    followingDeferred.resolve();
    await settled();
    assert.dom('[data-test-follow-button]').hasText('Follow').isEnabled();
    assert.dom('[data-test-follow-button] [data-test-spinner]').doesNotExist();

    let followDeferred = defer();
    this.worker.use(http.put('/api/v1/crates/:crate_id/follow', () => followDeferred.promise));

    click('[data-test-follow-button]');
    await waitFor('[data-test-follow-button] [data-test-spinner]');
    assert.dom('[data-test-follow-button]').hasText('Loading…').isDisabled();
    assert.dom('[data-test-follow-button] [data-test-spinner]').exists();

    followDeferred.resolve();
    await settled();
    assert.dom('[data-test-follow-button]').hasText('Unfollow').isEnabled();
    assert.dom('[data-test-follow-button] [data-test-spinner]').doesNotExist();

    let unfollowDeferred = defer();
    this.worker.use(http.delete('/api/v1/crates/:crate_id/follow', () => unfollowDeferred.promise));

    click('[data-test-follow-button]');
    await waitFor('[data-test-follow-button] [data-test-spinner]');
    assert.dom('[data-test-follow-button]').hasText('Loading…').isDisabled();
    assert.dom('[data-test-follow-button] [data-test-spinner]').exists();

    unfollowDeferred.resolve();
    await settled();
    assert.dom('[data-test-follow-button]').hasText('Follow').isEnabled();
    assert.dom('[data-test-follow-button] [data-test-spinner]').doesNotExist();
  });

  test('error handling when loading following state fails', async function (assert) {
    prepare(this);

    this.worker.use(http.get('/api/v1/crates/:crate_id/following', () => HttpResponse.json({}, { status: 500 })));

    await visit('/crates/nanomsg');
    assert.dom('[data-test-follow-button]').hasText('Follow').isDisabled();
    assert
      .dom('[data-test-notification-message="error"]')
      .hasText(
        'Something went wrong while trying to figure out if you are already following the nanomsg crate. Please try again later!',
      );
  });

  test('error handling when follow fails', async function (assert) {
    prepare(this);

    this.worker.use(http.put('/api/v1/crates/:crate_id/follow', () => HttpResponse.json({}, { status: 500 })));

    await visit('/crates/nanomsg');
    await click('[data-test-follow-button]');
    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Something went wrong when following the nanomsg crate. Please try again later!');
  });

  test('error handling when unfollow fails', async function (assert) {
    prepare(this, { following: true });

    this.worker.use(http.delete('/api/v1/crates/:crate_id/follow', () => HttpResponse.json({}, { status: 500 })));

    await visit('/crates/nanomsg');
    await click('[data-test-follow-button]');
    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Something went wrong when unfollowing the nanomsg crate. Please try again later!');
  });
});
