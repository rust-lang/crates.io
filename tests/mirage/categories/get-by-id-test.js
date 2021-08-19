import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/categories/:id', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown categories', async function (assert) {
    let response = await fetch('/api/v1/categories/foo');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns a category object for known categories', async function (assert) {
    this.server.create('category', {
      category: 'no-std',
      description: 'Crates that are able to function without the Rust standard library.',
    });

    let response = await fetch('/api/v1/categories/no-std');
    assert.equal(response.status, 200);
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

  test('calculates `crates_cnt` correctly', async function (assert) {
    this.server.create('category', { category: 'cli' });
    this.server.createList('crate', 7, { categoryIds: ['cli'] });
    this.server.create('category', { category: 'not-cli' });
    this.server.createList('crate', 3, { categoryIds: ['not-cli'] });

    let response = await fetch('/api/v1/categories/cli');
    assert.equal(response.status, 200);
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
});
