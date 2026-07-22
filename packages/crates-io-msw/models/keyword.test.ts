import { test } from 'vitest';

import { db } from '../index.js';

test('default are applied', async ({ expect }) => {
  let keyword = await db.keyword.create({});
  expect(keyword).toMatchInlineSnapshot(`
    {
      "created_at": "2010-06-16T21:30:45Z",
      "id": "keyword-1",
      "keyword": "keyword-1",
    }
  `);
});

test('name can be set', async ({ expect }) => {
  let keyword = await db.keyword.create({ keyword: 'gamedev' });
  expect(keyword).toMatchInlineSnapshot(`
    {
      "created_at": "2010-06-16T21:30:45Z",
      "id": "gamedev",
      "keyword": "gamedev",
    }
  `);
});
