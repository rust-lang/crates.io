import { test } from 'vitest';

import { db } from '../index.js';

test('default are applied', async ({ expect }) => {
  let crate = await db.crate.create({});
  expect(crate).toMatchInlineSnapshot(`
    {
      "_extra_downloads": [],
      "badges": [],
      "categories": [],
      "created_at": "2010-06-16T21:30:45Z",
      "description": "This is the description for the crate called "crate-1"",
      "documentation": null,
      "downloads": 37035,
      "homepage": null,
      "id": 1,
      "keywords": [],
      "name": "crate-1",
      "recent_downloads": 321,
      "repository": null,
      "trustpubOnly": false,
      "updated_at": "2017-02-24T12:34:56Z",
    }
  `);
});

test('attributes can be set', async ({ expect }) => {
  let category = await db.category.create({});
  let keyword1 = await db.keyword.create({});
  let keyword2 = await db.keyword.create({});

  let crate = await db.crate.create({
    name: 'crates-io',
    categories: [category],
    keywords: [keyword1, keyword2],
  });

  expect(crate).toMatchInlineSnapshot(`
    {
      "_extra_downloads": [],
      "badges": [],
      "categories": [
        {
          "category": "Category 1",
          "crates_cnt": null,
          "created_at": "2010-06-16T21:30:45Z",
          "description": "This is the description for the category called "Category 1"",
          "id": "category-1",
          "slug": "category-1",
        },
      ],
      "created_at": "2010-06-16T21:30:45Z",
      "description": "This is the description for the crate called "crates-io"",
      "documentation": null,
      "downloads": 37035,
      "homepage": null,
      "id": 1,
      "keywords": [
        {
          "id": "keyword-1",
          "keyword": "keyword-1",
        },
        {
          "id": "keyword-2",
          "keyword": "keyword-2",
        },
      ],
      "name": "crates-io",
      "recent_downloads": 321,
      "repository": null,
      "trustpubOnly": false,
      "updated_at": "2017-02-24T12:34:56Z",
    }
  `);
});
