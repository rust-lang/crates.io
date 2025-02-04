import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('empty case', async function () {
  let response = await fetch('/api/v1/keywords');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    keywords: [],
    meta: {
      total: 0,
    },
  });
});

test('returns a paginated keywords list', async function () {
  db.keyword.create({ keyword: 'api' });
  Array.from({ length: 2 }, () => db.keyword.create());

  let response = await fetch('/api/v1/keywords');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    keywords: [
      {
        id: 'api',
        crates_cnt: 0,
        keyword: 'api',
      },
      {
        id: 'keyword-2',
        crates_cnt: 0,
        keyword: 'keyword-2',
      },
      {
        id: 'keyword-3',
        crates_cnt: 0,
        keyword: 'keyword-3',
      },
    ],
    meta: {
      total: 3,
    },
  });
});

test('never returns more than 10 results', async function () {
  Array.from({ length: 25 }, () => db.keyword.create());

  let response = await fetch('/api/v1/keywords');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.keywords.length, 10);
  assert.strictEqual(responsePayload.meta.total, 25);
});

test('supports `page` and `per_page` parameters', async function () {
  Array.from({ length: 25 }, (_, i) => db.keyword.create({ keyword: `k${String(i + 1).padStart(2, '0')}` }));

  let response = await fetch('/api/v1/keywords?page=2&per_page=5');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.keywords.length, 5);
  assert.deepEqual(
    responsePayload.keywords.map(it => it.id),
    ['k06', 'k07', 'k08', 'k09', 'k10'],
  );
  assert.strictEqual(responsePayload.meta.total, 25);
});
