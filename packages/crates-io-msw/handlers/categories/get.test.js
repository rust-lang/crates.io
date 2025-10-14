import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown categories', async function () {
  let response = await fetch('/api/v1/categories/foo');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns a category object for known categories', async function () {
  await db.category.create({
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
  let testCli = await db.category.create({ category: 'test-cli-category' });
  await Promise.all(Array.from({ length: 7 }, () => db.crate.create({ categories: [testCli] })));
  let notTestCli = await db.category.create({ category: 'not-test-cli' });
  await Promise.all(Array.from({ length: 3 }, () => db.crate.create({ categories: [notTestCli] })));

  let response = await fetch('/api/v1/categories/test-cli-category');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    category: {
      category: 'test-cli-category',
      crates_cnt: 7,
      created_at: '2010-06-16T21:30:45Z',
      description: 'This is the description for the category called "test-cli-category"',
      id: 'test-cli-category',
      slug: 'test-cli-category',
    },
  });
});
