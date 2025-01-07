import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:name/:version/authors', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/1.0.0/authors');
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns 404 for unknown versions', async function (assert) {
    this.server.create('crate', { name: 'rand' });

    let response = await fetch('/api/v1/crates/rand/1.0.0/authors');
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'crate `rand` does not have a version `1.0.0`' }] });
  });

  test('empty case', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0' });

    let response = await fetch('/api/v1/crates/rand/1.0.0/authors');
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      meta: {
        names: [],
      },
      users: [],
    });
  });

  test('returns a list of authors belonging to the specified crate version', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0' });

    let response = await fetch('/api/v1/crates/rand/1.0.0/authors');
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      meta: {
        names: [],
      },
      users: [],
    });
  });
});
