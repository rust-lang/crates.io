import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('empty case', async function () {
  let response = await fetch('/api/v1/categories');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "categories": [],
      "meta": {
        "total": 0,
      },
    }
  `);
});

test('returns a paginated categories list', async function () {
  await db.category.create({
    category: 'no-std',
    description: 'Crates that are able to function without the Rust standard library.',
  });
  await Promise.all(Array.from({ length: 2 }, () => db.category.create({})));

  let response = await fetch('/api/v1/categories');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "categories": [
        {
          "category": "Category 2",
          "crates_cnt": 0,
          "created_at": "2010-06-16T21:30:45Z",
          "description": "This is the description for the category called "Category 2"",
          "id": "category-2",
          "slug": "category-2",
        },
        {
          "category": "Category 3",
          "crates_cnt": 0,
          "created_at": "2010-06-16T21:30:45Z",
          "description": "This is the description for the category called "Category 3"",
          "id": "category-3",
          "slug": "category-3",
        },
        {
          "category": "no-std",
          "crates_cnt": 0,
          "created_at": "2010-06-16T21:30:45Z",
          "description": "Crates that are able to function without the Rust standard library.",
          "id": "no-std",
          "slug": "no-std",
        },
      ],
      "meta": {
        "total": 3,
      },
    }
  `);
});

test('never returns more than 10 results', async function () {
  await Promise.all(Array.from({ length: 25 }, () => db.category.create({})));

  let response = await fetch('/api/v1/categories');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.categories.length).toBe(10);
  expect(responsePayload.meta.total).toBe(25);
});

test('supports `page` and `per_page` parameters', async function () {
  await Promise.all(
    Array.from({ length: 25 }, (_, i) =>
      db.category.create({
        category: `cat-${String(i + 1).padStart(2, '0')}`,
      }),
    ),
  );

  let response = await fetch('/api/v1/categories?page=2&per_page=5');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.categories.length).toBe(5);
  expect(responsePayload.categories.map(it => it.id)).toMatchInlineSnapshot(`
    [
      "cat-06",
      "cat-07",
      "cat-08",
      "cat-09",
      "cat-10",
    ]
  `);
  expect(responsePayload.meta.total).toBe(25);
});
