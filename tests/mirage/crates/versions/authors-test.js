import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:id/:version/authors', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/1.0.0/authors');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns 200 for unknown versions', async function (assert) {
    this.server.create('crate', { name: 'rand' });

    let response = await fetch('/api/v1/crates/rand/1.0.0/authors');
    // we should probably return 404 for this, but the production API
    // currently doesn't do this either
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'crate `rand` does not have a version `1.0.0`' }] });
  });

  test('empty case', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0' });

    let response = await fetch('/api/v1/crates/rand/1.0.0/authors');
    assert.equal(response.status, 200);
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
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      meta: {
        names: [],
      },
      users: [],
    });
  });
});
