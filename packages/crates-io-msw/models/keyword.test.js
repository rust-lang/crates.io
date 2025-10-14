import { test } from 'vitest';

import { db } from '../index.js';

test('default are applied', async ({ expect }) => {
  let keyword = await db.keyword.create();
  expect(keyword).toMatchInlineSnapshot(`
    {
      "id": "keyword-1",
      "keyword": "keyword-1",
    }
  `);
});

test('name can be set', async ({ expect }) => {
  let keyword = await db.keyword.create({ keyword: 'gamedev' });
  expect(keyword).toMatchInlineSnapshot(`
    {
      "id": "gamedev",
      "keyword": "gamedev",
    }
  `);
});
