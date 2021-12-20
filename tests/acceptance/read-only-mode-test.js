import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { AjaxError } from '../../utils/ajax';

module('Acceptance | Read-only Mode', function (hooks) {
  setupApplicationTest(hooks);

  test('notification is not shown for read-write mode', async function (assert) {
    await visit('/');
    assert.dom('[data-test-notification-message="info"]').doesNotExist();
  });

  test('notification is shown for read-only mode', async function (assert) {
    this.server.get('/api/v1/site_metadata', { read_only: true });

    await visit('/');
    assert.dom('[data-test-notification-message="info"]').includesText('read-only mode');
  });

  test('server errors are handled gracefully', async function (assert) {
    this.server.get('/api/v1/site_metadata', {}, 500);

    await visit('/');
    assert.dom('[data-test-notification-message="info"]').doesNotExist();
    assert.deepEqual(this.owner.lookup('service:sentry').events.length, 0);
  });

  test('client errors are reported on sentry', async function (assert) {
    this.server.get('/api/v1/site_metadata', {}, 404);

    await visit('/');
    assert.dom('[data-test-notification-message="info"]').doesNotExist();
    assert.deepEqual(this.owner.lookup('service:sentry').events.length, 1);
    assert.true(this.owner.lookup('service:sentry').events[0].error instanceof AjaxError);
  });
});
