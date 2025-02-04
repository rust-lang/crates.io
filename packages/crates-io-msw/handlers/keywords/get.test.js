import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown keywords', async function () {
  let response = await fetch('/api/v1/keywords/foo');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns a keyword object for known keywords', async function () {
  db.keyword.create({ keyword: 'cli' });

  let response = await fetch('/api/v1/keywords/cli');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    keyword: {
      id: 'cli',
      crates_cnt: 0,
      keyword: 'cli',
    },
  });
});

test('calculates `crates_cnt` correctly', async function () {
  let cli = db.keyword.create({ keyword: 'cli' });
  Array.from({ length: 7 }, () => db.crate.create({ keywords: [cli] }));
  let notCli = db.keyword.create({ keyword: 'not-cli' });
  Array.from({ length: 3 }, () => db.crate.create({ keywords: [notCli] }));

  let response = await fetch('/api/v1/keywords/cli');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    keyword: {
      id: 'cli',
      crates_cnt: 7,
      keyword: 'cli',
    },
  });
});
