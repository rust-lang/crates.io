import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('empty case', async function () {
  let response = await fetch('/api/v1/categories');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    categories: [],
    meta: {
      total: 0,
    },
  });
});

test('returns a paginated categories list', async function () {
  db.category.create({
    category: 'no-std',
    description: 'Crates that are able to function without the Rust standard library.',
  });
  Array.from({ length: 2 }, () => db.category.create());

  let response = await fetch('/api/v1/categories');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    categories: [
      {
        id: 'category-2',
        category: 'Category 2',
        crates_cnt: 0,
        created_at: '2010-06-16T21:30:45Z',
        description: 'This is the description for the category called "Category 2"',
        slug: 'category-2',
      },
      {
        id: 'category-3',
        category: 'Category 3',
        crates_cnt: 0,
        created_at: '2010-06-16T21:30:45Z',
        description: 'This is the description for the category called "Category 3"',
        slug: 'category-3',
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

test('never returns more than 10 results', async function () {
  Array.from({ length: 25 }, () => db.category.create());

  let response = await fetch('/api/v1/categories');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.categories.length, 10);
  assert.strictEqual(responsePayload.meta.total, 25);
});

test('supports `page` and `per_page` parameters', async function () {
  Array.from({ length: 25 }, (_, i) =>
    db.category.create({
      category: `cat-${String(i + 1).padStart(2, '0')}`,
    }),
  );

  let response = await fetch('/api/v1/categories?page=2&per_page=5');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.categories.length, 5);
  assert.deepEqual(
    responsePayload.categories.map(it => it.id),
    ['cat-06', 'cat-07', 'cat-08', 'cat-09', 'cat-10'],
  );
  assert.strictEqual(responsePayload.meta.total, 25);
});
