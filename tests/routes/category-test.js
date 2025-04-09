import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Route | category', function (hooks) {
  setupApplicationTest(hooks);

  test("shows an error message if the category can't be found", async function (assert) {
    await visit('/categories/foo');
    assert.strictEqual(currentURL(), '/categories/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Category not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('server error causes the error page to be shown', async function (assert) {
    let error = HttpResponse.json({}, { status: 500 });
    this.worker.use(http.get('/api/v1/categories/:categoryId', () => error));

    await visit('/categories/foo');
    assert.strictEqual(currentURL(), '/categories/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load category data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });

  test('updates the search field when the categories route is accessed', async function (assert) {
    this.db.category.create({ category: 'foo' });

    await visit('/');
    assert.dom('[data-test-search-input]').hasValue('');

    await visit('/categories/foo');
    assert.dom('[data-test-search-input]').hasValue('category:foo ');

    await visit('/');
    assert.dom('[data-test-search-input]').hasValue('');
  });
});
