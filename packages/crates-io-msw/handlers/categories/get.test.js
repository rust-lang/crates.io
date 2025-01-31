import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown categories', async function () {
  let response = await fetch('/api/v1/categories/foo');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns a category object for known categories', async function () {
  db.category.create({
    category: 'no-std',
    description: 'Crates that are able to function without the Rust standard library.',
  });

  let response = await fetch('/api/v1/categories/no-std');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    category: {
      id: 'no-std',
      category: 'no-std',
      crates_cnt: 0,
      created_at: '2010-06-16T21:30:45Z',
      description: 'Crates that are able to function without the Rust standard library.',
      slug: 'no-std',
    },
  });
});

test('calculates `crates_cnt` correctly', async function () {
  let cli = db.category.create({ category: 'cli' });
  Array.from({ length: 7 }, () => db.crate.create({ categories: [cli] }));
  let notCli = db.category.create({ category: 'not-cli' });
  Array.from({ length: 3 }, () => db.crate.create({ categories: [notCli] }));

  let response = await fetch('/api/v1/categories/cli');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    category: {
      category: 'cli',
      crates_cnt: 7,
      created_at: '2010-06-16T21:30:45Z',
      description: 'This is the description for the category called "cli"',
      id: 'cli',
      slug: 'cli',
    },
  });
});
