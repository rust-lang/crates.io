import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Route | keyword', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  test('shows an empty list if the keyword does not exist on the server', async function (assert) {
    await visit('/keywords/foo');
    assert.strictEqual(currentURL(), '/keywords/foo');
    assert.dom('[data-test-crate-row]').doesNotExist();
  });

  test('server error causes the error page to be shown', async function (assert) {
    let error = HttpResponse.json({}, { status: 500 });
    this.worker.use(http.get('/api/v1/crates', () => error));

    await visit('/keywords/foo');
    assert.strictEqual(currentURL(), '/keywords/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load crates');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });
});
