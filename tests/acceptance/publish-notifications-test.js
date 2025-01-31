import { click, currentURL, visit, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

module('Acceptance | publish notifications', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  test('unsubscribe and resubscribe', async function (assert) {
    let user = this.db.user.create();

    this.authenticateAs(user);
    assert.true(user.publishNotifications);

    await visit('/settings/profile');
    assert.strictEqual(currentURL(), '/settings/profile');
    assert.dom('[data-test-notifications] input[type=checkbox]').isChecked();

    await click('[data-test-notifications] input[type=checkbox]');
    assert.dom('[data-test-notifications] input[type=checkbox]').isNotChecked();

    await click('[data-test-notifications] button');
    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.false(user.publishNotifications);

    await click('[data-test-notifications] input[type=checkbox]');
    assert.dom('[data-test-notifications] input[type=checkbox]').isChecked();

    await click('[data-test-notifications] button');
    user = this.db.user.findFirst({ where: { id: { equals: user.id } } });
    assert.true(user.publishNotifications);
  });

  test('loading and error state', async function (assert) {
    let user = this.db.user.create();

    let deferred = defer();
    this.worker.use(http.put('/api/v1/users/:user_id', () => deferred.promise));

    this.authenticateAs(user);
    assert.true(user.publishNotifications);

    await visit('/settings/profile');
    assert.strictEqual(currentURL(), '/settings/profile');

    await click('[data-test-notifications] input[type=checkbox]');

    let clickPromise = click('[data-test-notifications] button');
    await waitFor('[data-test-notifications] [data-test-spinner]');
    assert.dom('[data-test-notifications] input[type=checkbox]').isDisabled();
    assert.dom('[data-test-notifications] button').isDisabled();

    deferred.resolve(HttpResponse.json({}, { status: 500 }));
    await clickPromise;

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Something went wrong while updating your notification settings. Please try again later!');
  });
});
