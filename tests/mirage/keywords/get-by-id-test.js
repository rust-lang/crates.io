import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/keywords/:id', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown keywords', async function (assert) {
    let response = await fetch('/api/v1/keywords/foo');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns a keyword object for known keywords', async function (assert) {
    this.server.create('keyword', { keyword: 'cli' });

    let response = await fetch('/api/v1/keywords/cli');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      keyword: {
        id: 'cli',
        crates_cnt: 0,
        keyword: 'cli',
      },
    });
  });

  test('calculates `crates_cnt` correctly', async function (assert) {
    this.server.create('keyword', { keyword: 'cli' });
    this.server.createList('crate', 7, { keywordIds: ['cli'] });
    this.server.create('keyword', { keyword: 'not-cli' });
    this.server.createList('crate', 3, { keywordIds: ['not-cli'] });

    let response = await fetch('/api/v1/keywords/cli');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      keyword: {
        id: 'cli',
        crates_cnt: 7,
        keyword: 'cli',
      },
    });
  });
});
