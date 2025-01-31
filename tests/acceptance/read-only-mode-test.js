import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { AjaxError } from '../../utils/ajax';

module('Acceptance | Read-only Mode', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  test('notification is not shown for read-write mode', async function (assert) {
    await visit('/');
    assert.dom('[data-test-notification-message="info"]').doesNotExist();
  });

  test('notification is shown for read-only mode', async function (assert) {
    this.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({ read_only: true })));

    await visit('/');
    assert.dom('[data-test-notification-message="info"]').includesText('read-only mode');
  });

  test('server errors are handled gracefully', async function (assert) {
    this.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({}, { status: 500 })));

    await visit('/');
    assert.dom('[data-test-notification-message="info"]').doesNotExist();
    assert.deepEqual(this.owner.lookup('service:sentry').events.length, 0);
  });

  test('client errors are reported on sentry', async function (assert) {
    this.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({}, { status: 404 })));

    await visit('/');
    assert.dom('[data-test-notification-message="info"]').doesNotExist();
    assert.deepEqual(this.owner.lookup('service:sentry').events.length, 1);
    assert.true(this.owner.lookup('service:sentry').events[0].error instanceof AjaxError);
  });
});
