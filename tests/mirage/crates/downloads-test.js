import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:id/downloads', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/downloads');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('empty case', async function (assert) {
    this.server.create('crate', { name: 'rand' });

    let response = await fetch('/api/v1/crates/rand/downloads');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      version_downloads: [],
      meta: {
        extra_downloads: [],
      },
    });
  });

  test('returns a list of version downloads belonging to the specified crate version', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    let versions = this.server.createList('version', 2, { crate });
    this.server.create('version-download', { version: versions[0], date: '2020-01-13' });
    this.server.create('version-download', { version: versions[1], date: '2020-01-14' });
    this.server.create('version-download', { version: versions[1], date: '2020-01-15' });

    let response = await fetch('/api/v1/crates/rand/downloads');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      version_downloads: [
        {
          date: '2020-01-13',
          downloads: 9380,
          version: '1',
        },
        {
          date: '2020-01-14',
          downloads: 16_415,
          version: '2',
        },
        {
          date: '2020-01-15',
          downloads: 23_450,
          version: '2',
        },
      ],
      meta: {
        extra_downloads: [],
      },
    });
  });
});
