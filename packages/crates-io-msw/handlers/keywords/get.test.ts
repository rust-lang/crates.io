import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown keywords', async function () {
  let response = await fetch('/api/v1/keywords/foo');
  expect(response.status).toBe(404);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "Not Found",
        },
      ],
    }
  `);
});

test('returns a keyword object for known keywords', async function () {
  await db.keyword.create({ keyword: 'cli' });

  let response = await fetch('/api/v1/keywords/cli');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "keyword": {
        "crates_cnt": 0,
        "id": "cli",
        "keyword": "cli",
      },
    }
  `);
});

test('calculates `crates_cnt` correctly', async function () {
  let testKeyword = await db.keyword.create({ keyword: 'test-cli-keyword' });
  await Promise.all(Array.from({ length: 7 }, () => db.crate.create({ keywords: [testKeyword] })));
  let notTestKeyword = await db.keyword.create({ keyword: 'not-test-cli' });
  await Promise.all(Array.from({ length: 3 }, () => db.crate.create({ keywords: [notTestKeyword] })));

  let response = await fetch('/api/v1/keywords/test-cli-keyword');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "keyword": {
        "crates_cnt": 7,
        "id": "test-cli-keyword",
        "keyword": "test-cli-keyword",
      },
    }
  `);
});
