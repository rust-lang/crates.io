import { test } from 'vitest';

import { db } from '../index.js';

test('default are applied', ({ expect }) => {
  let category = db.category.create();
  expect(category).toMatchInlineSnapshot(`
    {
      "category": "Category 1",
      "crates_cnt": null,
      "created_at": "2010-06-16T21:30:45Z",
      "description": "This is the description for the category called "Category 1"",
      "id": "category-1",
      "slug": "category-1",
      Symbol(type): "category",
      Symbol(primaryKey): "id",
    }
  `);
});

test('name can be set', ({ expect }) => {
  let category = db.category.create({ category: 'Network programming' });
  expect(category).toMatchInlineSnapshot(`
    {
      "category": "Network programming",
      "crates_cnt": null,
      "created_at": "2010-06-16T21:30:45Z",
      "description": "This is the description for the category called "Network programming"",
      "id": "network-programming",
      "slug": "network-programming",
      Symbol(type): "category",
      Symbol(primaryKey): "id",
    }
  `);
});
