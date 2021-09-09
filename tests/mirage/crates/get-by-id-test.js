import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:id', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns a crate object for known crates', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0-beta.1' });

    let response = await fetch('/api/v1/crates/rand');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      categories: [],
      crate: {
        badges: [],
        categories: [],
        created_at: '2010-06-16T21:30:45Z',
        description: 'This is the description for the crate called "rand"',
        documentation: null,
        downloads: 0,
        homepage: null,
        id: 'rand',
        keywords: [],
        links: {
          owner_team: '/api/v1/crates/rand/owner_team',
          owner_user: '/api/v1/crates/rand/owner_user',
          reverse_dependencies: '/api/v1/crates/rand/reverse_dependencies',
          version_downloads: '/api/v1/crates/rand/downloads',
          versions: '/api/v1/crates/rand/versions',
        },
        max_version: '1.0.0-beta.1',
        max_stable_version: null,
        name: 'rand',
        newest_version: '1.0.0-beta.1',
        repository: null,
        updated_at: '2017-02-24T12:34:56Z',
        versions: ['1'],
      },
      keywords: [],
      versions: [
        {
          id: '1',
          crate: 'rand',
          crate_size: 0,
          created_at: '2010-06-16T21:30:45Z',
          dl_path: '/api/v1/crates/rand/1.0.0-beta.1/download',
          downloads: 0,
          license: 'MIT/Apache-2.0',
          links: {
            dependencies: '/api/v1/crates/rand/1.0.0-beta.1/dependencies',
            version_downloads: '/api/v1/crates/rand/1.0.0-beta.1/downloads',
          },
          num: '1.0.0-beta.1',
          published_by: null,
          readme_path: '/api/v1/crates/rand/1.0.0-beta.1/readme',
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
        },
      ],
    });
  });

  test('includes related versions', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0' });
    this.server.create('version', { crate, num: '1.2.0' });

    let response = await fetch('/api/v1/crates/rand');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.deepEqual(responsePayload.crate.versions, ['1', '2', '3']);
    assert.deepEqual(responsePayload.versions, [
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
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
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
        published_by: null,
        readme_path: '/api/v1/crates/rand/1.1.0/readme',
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
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
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
      },
    ]);
  });

  test('includes related categories', async function (assert) {
    this.server.create('category', { category: 'no-std' });
    this.server.create('category', { category: 'cli' });
    let crate = this.server.create('crate', { name: 'rand', categoryIds: ['no-std'] });
    this.server.create('version', { crate });

    let response = await fetch('/api/v1/crates/rand');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.deepEqual(responsePayload.crate.categories, ['no-std']);
    assert.deepEqual(responsePayload.categories, [
      {
        id: 'no-std',
        category: 'no-std',
        crates_cnt: 1,
        created_at: '2010-06-16T21:30:45Z',
        description: 'This is the description for the category called "no-std"',
        slug: 'no-std',
      },
    ]);
  });

  test('includes related keywords', async function (assert) {
    this.server.create('keyword', { keyword: 'no-std' });
    this.server.create('keyword', { keyword: 'cli' });
    let crate = this.server.create('crate', { name: 'rand', keywordIds: ['no-std'] });
    this.server.create('version', { crate });

    let response = await fetch('/api/v1/crates/rand');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.deepEqual(responsePayload.crate.keywords, ['no-std']);
    assert.deepEqual(responsePayload.keywords, [
      {
        crates_cnt: 1,
        id: 'no-std',
        keyword: 'no-std',
      },
    ]);
  });
});
