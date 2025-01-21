import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('empty case', async function () {
  let response = await fetch('/api/v1/category_slugs');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    category_slugs: [],
  });
});

test('returns a category slugs list', async function () {
  db.category.create({
    category: 'no-std',
    description: 'Crates that are able to function without the Rust standard library.',
  });
  Array.from({ length: 2 }, () => db.category.create());

  let response = await fetch('/api/v1/category_slugs');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    category_slugs: [
      {
        description: 'This is the description for the category called "Category 2"',
        id: 'category-2',
        slug: 'category-2',
      },
      {
        description: 'This is the description for the category called "Category 3"',
        id: 'category-3',
        slug: 'category-3',
      },
      {
        description: 'Crates that are able to function without the Rust standard library.',
        id: 'no-std',
        slug: 'no-std',
      },
    ],
  });
});

test('has no pagination', async function () {
  Array.from({ length: 25 }, () => db.category.create());

  let response = await fetch('/api/v1/category_slugs');
  assert.strictEqual(response.status, 200);
  assert.strictEqual((await response.json()).category_slugs.length, 25);
});
