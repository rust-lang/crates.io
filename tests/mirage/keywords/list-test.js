import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/keywords', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('empty case', async function (assert) {
    let response = await fetch('/api/v1/keywords');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      keywords: [],
      meta: {
        total: 0,
      },
    });
  });

  test('returns a paginated keywords list', async function (assert) {
    this.server.create('keyword', { keyword: 'api' });
    this.server.createList('keyword', 2);

    let response = await fetch('/api/v1/keywords');
    assert.equal(response.status, 200);
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

  test('never returns more than 10 results', async function (assert) {
    this.server.createList('keyword', 25);

    let response = await fetch('/api/v1/keywords');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.keywords.length, 10);
    assert.equal(responsePayload.meta.total, 25);
  });

  test('supports `page` and `per_page` parameters', async function (assert) {
    this.server.createList('keyword', 25, {
      keyword: i => `k${String(i + 1).padStart(2, '0')}`,
    });

    let response = await fetch('/api/v1/keywords?page=2&per_page=5');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.keywords.length, 5);
    assert.deepEqual(
      responsePayload.keywords.map(it => it.id),
      ['k06', 'k07', 'k08', 'k09', 'k10'],
    );
    assert.equal(responsePayload.meta.total, 25);
  });
});
