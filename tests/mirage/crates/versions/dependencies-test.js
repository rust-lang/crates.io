import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:id/:version/dependencies', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/1.0.0/dependencies');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns 200 for unknown versions', async function (assert) {
    this.server.create('crate', { name: 'rand' });

    let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
    // we should probably return 404 for this, but the production API
    // currently doesn't do this either
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'crate `rand` does not have a version `1.0.0`' }] });
  });

  test('empty case', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0' });

    let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      dependencies: [],
    });
  });

  test('returns a list of dependencies belonging to the specified crate version', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    let version = this.server.create('version', { crate, num: '1.0.0' });

    let foo = this.server.create('crate', { name: 'foo' });
    this.server.create('dependency', { crate: foo, version });
    let bar = this.server.create('crate', { name: 'bar' });
    this.server.create('dependency', { crate: bar, version });
    let baz = this.server.create('crate', { name: 'baz' });
    this.server.create('dependency', { crate: baz, version });

    let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      dependencies: [
        {
          id: '1',
          crate_id: 'foo',
          default_features: false,
          features: [],
          kind: 'dev',
          optional: true,
          req: '^0.1.0',
          target: null,
          version_id: '1',
        },
        {
          id: '2',
          crate_id: 'bar',
          default_features: false,
          features: [],
          kind: 'normal',
          optional: true,
          req: '^2.1.3',
          target: null,
          version_id: '1',
        },
        {
          id: '3',
          crate_id: 'baz',
          default_features: false,
          features: [],
          kind: 'normal',
          optional: true,
          req: '0.3.7',
          target: null,
          version_id: '1',
        },
      ],
    });
  });
});
