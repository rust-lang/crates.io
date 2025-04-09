import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Route | user', function (hooks) {
  setupApplicationTest(hooks);

  test("shows an error message if the user can't be found", async function (assert) {
    await visit('/users/foo');
    assert.strictEqual(currentURL(), '/users/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: User not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('server error causes the error page to be shown', async function (assert) {
    let error = HttpResponse.json({}, { status: 500 });
    this.worker.use(http.get('/api/v1/users/:id', () => error));

    await visit('/users/foo');
    assert.strictEqual(currentURL(), '/users/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load user data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });
});
