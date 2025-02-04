import { test } from 'vitest';

import { db } from '../index.js';

test('default are applied', ({ expect }) => {
  let keyword = db.keyword.create();
  expect(keyword).toMatchInlineSnapshot(`
    {
      "id": "keyword-1",
      "keyword": "keyword-1",
      Symbol(type): "keyword",
      Symbol(primaryKey): "id",
    }
  `);
});

test('name can be set', ({ expect }) => {
  let keyword = db.keyword.create({ keyword: 'gamedev' });
  expect(keyword).toMatchInlineSnapshot(`
    {
      "id": "gamedev",
      "keyword": "gamedev",
      Symbol(type): "keyword",
      Symbol(primaryKey): "id",
    }
  `);
});
