import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/categories', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('empty case', async function (assert) {
    let response = await fetch('/api/v1/categories');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      categories: [],
      meta: {
        total: 0,
      },
    });
  });

  test('returns a paginated categories list', async function (assert) {
    this.server.create('category', {
      category: 'no-std',
      description: 'Crates that are able to function without the Rust standard library.',
    });
    this.server.createList('category', 2);

    let response = await fetch('/api/v1/categories');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      categories: [
        {
          id: 'category-1',
          category: 'Category 1',
          crates_cnt: 0,
          created_at: '2010-06-16T21:30:45Z',
          description: 'This is the description for the category called "Category 1"',
          slug: 'category-1',
        },
        {
          id: 'category-2',
          category: 'Category 2',
          crates_cnt: 0,
          created_at: '2010-06-16T21:30:45Z',
          description: 'This is the description for the category called "Category 2"',
          slug: 'category-2',
        },
        {
          id: 'no-std',
          category: 'no-std',
          crates_cnt: 0,
          created_at: '2010-06-16T21:30:45Z',
          description: 'Crates that are able to function without the Rust standard library.',
          slug: 'no-std',
        },
      ],
      meta: {
        total: 3,
      },
    });
  });

  test('never returns more than 10 results', async function (assert) {
    this.server.createList('category', 25);

    let response = await fetch('/api/v1/categories');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.categories.length, 10);
    assert.equal(responsePayload.meta.total, 25);
  });

  test('supports `page` and `per_page` parameters', async function (assert) {
    this.server.createList('category', 25, {
      category: i => `cat-${String(i + 1).padStart(2, '0')}`,
    });

    let response = await fetch('/api/v1/categories?page=2&per_page=5');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.categories.length, 5);
    assert.deepEqual(
      responsePayload.categories.map(it => it.id),
      ['cat-06', 'cat-07', 'cat-08', 'cat-09', 'cat-10'],
    );
    assert.equal(responsePayload.meta.total, 25);
  });
});
