import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Route | category', function (hooks) {
  setupApplicationTest(hooks);

  test("shows an error message if the category can't be found", async function (assert) {
    await visit('/categories/unknown');
    assert.equal(currentURL(), '/');
    assert.dom('[data-test-notification-message]').hasText("Category 'unknown' does not exist");
  });

  test('server error causes the error page to be shown', async function (assert) {
    this.server.get('/api/v1/categories/:categoryId', {}, 500);

    await visit('/categories/error');
    assert.equal(currentURL(), '/categories/error');
    assert.dom('[data-test-error-message]').includesText('GET /api/v1/categories/error returned a 500');
  });
});
