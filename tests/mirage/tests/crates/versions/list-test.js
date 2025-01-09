import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../../helpers';
import setupMirage from '../../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:name/versions', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/versions');
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('empty case', async function (assert) {
    this.server.create('crate', { name: 'rand' });

    let response = await fetch('/api/v1/crates/rand/versions');
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      versions: [],
      meta: { total: 0, next_page: null },
    });
  });

  test('returns all versions belonging to the specified crate', async function (assert) {
    let user = this.server.create('user');
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0', publishedBy: user });
    this.server.create('version', { crate, num: '1.2.0', rust_version: '1.69' });

    let response = await fetch('/api/v1/crates/rand/versions');
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      versions: [
        {
          id: '1',
          crate: 'rand',
          crate_size: 0,
          created_at: '2010-06-16T21:30:45Z',
          dl_path: '/api/v1/crates/rand/1.0.0/download',
          downloads: 0,
          license: 'MIT/Apache-2.0',
          links: {
            dependencies: '/api/v1/crates/rand/1.0.0/dependencies',
            version_downloads: '/api/v1/crates/rand/1.0.0/downloads',
          },
          num: '1.0.0',
          published_by: null,
          readme_path: '/api/v1/crates/rand/1.0.0/readme',
          rust_version: null,
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
          yank_message: null,
        },
        {
          id: '2',
          crate: 'rand',
          crate_size: 162_963,
          created_at: '2010-06-16T21:30:45Z',
          dl_path: '/api/v1/crates/rand/1.1.0/download',
          downloads: 3702,
          license: 'MIT',
          links: {
            dependencies: '/api/v1/crates/rand/1.1.0/dependencies',
            version_downloads: '/api/v1/crates/rand/1.1.0/downloads',
          },
          num: '1.1.0',
          published_by: {
            id: 1,
            avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
            login: 'user-1',
            name: 'User 1',
            url: 'https://github.com/user-1',
          },
          readme_path: '/api/v1/crates/rand/1.1.0/readme',
          rust_version: null,
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
          yank_message: null,
        },
        {
          id: '3',
          crate: 'rand',
          crate_size: 325_926,
          created_at: '2010-06-16T21:30:45Z',
          dl_path: '/api/v1/crates/rand/1.2.0/download',
          downloads: 7404,
          license: 'Apache-2.0',
          links: {
            dependencies: '/api/v1/crates/rand/1.2.0/dependencies',
            version_downloads: '/api/v1/crates/rand/1.2.0/downloads',
          },
          num: '1.2.0',
          published_by: null,
          readme_path: '/api/v1/crates/rand/1.2.0/readme',
          rust_version: '1.69',
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
          yank_message: null,
        },
      ],
      meta: { total: 3, next_page: null },
    });
  });

  test('supports multiple `ids[]` parameters', async function (assert) {
    let user = this.server.create('user');
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0', publishedBy: user });
    this.server.create('version', { crate, num: '1.2.0', rust_version: '1.69' });
    let response = await fetch('/api/v1/crates/rand/versions?nums[]=1.0.0&nums[]=1.2.0');
    assert.strictEqual(response.status, 200);
    let json = await response.json();
    assert.deepEqual(
      json.versions.map(v => v.num),
      ['1.0.0', '1.2.0'],
    );
  });

  test('include `release_tracks` meta', async function (assert) {
    let user = this.server.create('user');
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '0.0.1' });
    this.server.create('version', { crate, num: '0.0.2', yanked: true });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0', publishedBy: user });
    this.server.create('version', { crate, num: '1.2.0', rust_version: '1.69', yanked: true });

    let req = await fetch('/api/v1/crates/rand/versions');
    let expected = await req.json();

    let response = await fetch('/api/v1/crates/rand/versions?include=release_tracks');
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      ...expected,
      meta: {
        ...expected.meta,
        release_tracks: {
          '0.0': {
            highest: '0.0.1',
          },
          1: {
            highest: '1.1.0',
          },
        },
      },
    });
  });
});
