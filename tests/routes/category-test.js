import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Route | category', function (hooks) {
  setupApplicationTest(hooks);

  test("shows an error message if the category can't be found", async function (assert) {
    await visit('/categories/foo');
    assert.equal(currentURL(), '/categories/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Category not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('server error causes the error page to be shown', async function (assert) {
    this.server.get('/api/v1/categories/:categoryId', {}, 500);

    await visit('/categories/foo');
    assert.equal(currentURL(), '/categories/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load category data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });
});
