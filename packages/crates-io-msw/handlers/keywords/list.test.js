import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('empty case', async function () {
  let response = await fetch('/api/v1/keywords');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "keywords": [],
      "meta": {
        "total": 0,
      },
    }
  `);
});

test('returns a paginated keywords list', async function () {
  await db.keyword.create({ keyword: 'api' });
  await Promise.all(Array.from({ length: 2 }, () => db.keyword.create()));

  let response = await fetch('/api/v1/keywords');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "keywords": [
        {
          "crates_cnt": 0,
          "id": "api",
          "keyword": "api",
        },
        {
          "crates_cnt": 0,
          "id": "keyword-2",
          "keyword": "keyword-2",
        },
        {
          "crates_cnt": 0,
          "id": "keyword-3",
          "keyword": "keyword-3",
        },
      ],
      "meta": {
        "total": 3,
      },
    }
  `);
});

test('never returns more than 10 results', async function () {
  await Promise.all(Array.from({ length: 25 }, () => db.keyword.create()));

  let response = await fetch('/api/v1/keywords');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.keywords.length).toBe(10);
  expect(responsePayload.meta.total).toBe(25);
});

test('supports `page` and `per_page` parameters', async function () {
  await Promise.all(
    Array.from({ length: 25 }, (_, i) => db.keyword.create({ keyword: `k${String(i + 1).padStart(2, '0')}` })),
  );

  let response = await fetch('/api/v1/keywords?page=2&per_page=5');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.keywords.length).toBe(5);
  expect(responsePayload.keywords.map(it => it.id)).toMatchInlineSnapshot(`
    [
      "k06",
      "k07",
      "k08",
      "k09",
      "k10",
    ]
  `);
  expect(responsePayload.meta.total).toBe(25);
});
