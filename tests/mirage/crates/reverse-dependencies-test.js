import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:id/reverse_dependencies', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('empty case', async function (assert) {
    this.server.create('crate', { name: 'rand' });

    let response = await fetch('/api/v1/crates/rand/reverse_dependencies');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      dependencies: [],
      versions: [],
      meta: {
        total: 0,
      },
    });
  });

  test('returns a paginated list of crate versions depending to the specified crate', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });

    this.server.create('dependency', {
      crate,
      version: this.server.create('version', {
        crate: this.server.create('crate', { name: 'bar' }),
      }),
    });

    this.server.create('dependency', {
      crate,
      version: this.server.create('version', {
        crate: this.server.create('crate', { name: 'baz' }),
      }),
    });

    let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
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
          crate_id: 'foo',
          default_features: false,
          features: [],
          kind: 'normal',
          optional: true,
          req: '^2.1.3',
          target: null,
          version_id: '2',
        },
      ],
      versions: [
        {
          id: '1',
          crate: 'bar',
          crate_size: 0,
          created_at: '2010-06-16T21:30:45Z',
          dl_path: '/api/v1/crates/bar/1.0.0/download',
          downloads: 0,
          license: 'MIT/Apache-2.0',
          links: {
            dependencies: '/api/v1/crates/bar/1.0.0/dependencies',
            version_downloads: '/api/v1/crates/bar/1.0.0/downloads',
          },
          num: '1.0.0',
          published_by: null,
          readme_path: '/api/v1/crates/bar/1.0.0/readme',
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
        },
        {
          id: '2',
          crate: 'baz',
          crate_size: 162_963,
          created_at: '2010-06-16T21:30:45Z',
          dl_path: '/api/v1/crates/baz/1.0.1/download',
          downloads: 3702,
          license: 'MIT',
          links: {
            dependencies: '/api/v1/crates/baz/1.0.1/dependencies',
            version_downloads: '/api/v1/crates/baz/1.0.1/downloads',
          },
          num: '1.0.1',
          published_by: null,
          readme_path: '/api/v1/crates/baz/1.0.1/readme',
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
        },
      ],
      meta: {
        total: 2,
      },
    });
  });

  test('never returns more than 10 results', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });

    this.server.createList('dependency', 25, {
      crate,
      version: () =>
        this.server.create('version', {
          crate: () => this.server.create('crate', { name: 'bar' }),
        }),
    });

    let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.dependencies.length, 10);
    assert.equal(responsePayload.versions.length, 10);
    assert.equal(responsePayload.meta.total, 25);
  });

  test('supports `page` and `per_page` parameters', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });

    let crates = this.server.createList('crate', 25, {
      name: i => `crate-${String(i + 1).padStart(2, '0')}`,
    });
    let versions = this.server.createList('version', crates.length, {
      crate: i => crates[i],
    });
    this.server.createList('dependency', versions.length, {
      crate,
      versionId: i => versions[i].id,
    });

    let response = await fetch('/api/v1/crates/foo/reverse_dependencies?page=2&per_page=5');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.dependencies.length, 5);
    assert.deepEqual(
      responsePayload.versions.map(it => it.crate),
      // offset by one because we created the `foo` crate first
      ['crate-07', 'crate-08', 'crate-09', 'crate-10', 'crate-11'],
    );
    assert.equal(responsePayload.meta.total, 25);
  });
});
