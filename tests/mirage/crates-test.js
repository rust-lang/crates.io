import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from '../helpers/setup-mirage';
import fetch from 'fetch';

module('Mirage | Crates', function(hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('GET /api/v1/crates', function() {
    test('empty case', async function(assert) {
      let response = await fetch('/api/v1/crates');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        crates: [],
        meta: {
          total: 0,
        },
      });
    });

    test('returns a paginated crates list', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crateId: 'rand', num: '1.0.0' });

      let response = await fetch('/api/v1/crates');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        crates: [
          {
            id: 'rand',
            badges: [],
            categories: [],
            created_at: '2010-06-16T21:30:45Z',
            description: 'This is the description for the crate called "rand"',
            documentation: null,
            downloads: 0,
            homepage: null,
            keywords: [],
            links: {
              owner_team: '/api/v1/crates/rand/owner_team',
              owner_user: '/api/v1/crates/rand/owner_user',
              reverse_dependencies: '/api/v1/crates/rand/reverse_dependencies',
              version_downloads: '/api/v1/crates/rand/downloads',
              versions: '/api/v1/crates/rand/versions',
            },
            max_version: '1.0.0',
            name: 'rand',
            newest_version: '1.0.0',
            repository: null,
            updated_at: '2017-02-24T12:34:56Z',
            versions: ['1'],
          },
        ],
        meta: {
          total: 1,
        },
      });
    });

    test('never returns more than 10 results', async function(assert) {
      let crates = this.server.createList('crate', 25);
      this.server.createList('version', crates.length, { crate: i => crates[i] });

      let response = await fetch('/api/v1/crates');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 10);
      assert.equal(responsePayload.meta.total, 25);
    });

    test('supports `page` and `per_page` parameters', async function(assert) {
      let crates = this.server.createList('crate', 25, {
        name: i => `crate-${String(i + 1).padStart(2, '0')}`,
      });
      this.server.createList('version', crates.length, { crate: i => crates[i] });

      let response = await fetch('/api/v1/crates?page=2&per_page=5');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 5);
      assert.deepEqual(
        responsePayload.crates.map(it => it.id),
        ['crate-06', 'crate-07', 'crate-08', 'crate-09', 'crate-10'],
      );
      assert.equal(responsePayload.meta.total, 25);
    });

    test('supports a `letter` parameter', async function(assert) {
      this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crateId: 'foo' });
      this.server.create('crate', { name: 'bar' });
      this.server.create('version', { crateId: 'bar' });
      this.server.create('crate', { name: 'BAZ' });
      this.server.create('version', { crateId: 'BAZ' });

      let response = await fetch('/api/v1/crates?letter=b');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 2);
      assert.deepEqual(
        responsePayload.crates.map(it => it.id),
        ['bar', 'BAZ'],
      );
      assert.equal(responsePayload.meta.total, 2);
    });

    test('supports a `q` parameter', async function(assert) {
      this.server.create('crate', { name: '123456' });
      this.server.create('version', { crateId: '123456' });
      this.server.create('crate', { name: '00123' });
      this.server.create('version', { crateId: '00123' });
      this.server.create('crate', { name: '87654' });
      this.server.create('version', { crateId: '87654' });

      let response = await fetch('/api/v1/crates?q=123');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 2);
      assert.deepEqual(
        responsePayload.crates.map(it => it.id),
        ['123456', '00123'],
      );
      assert.equal(responsePayload.meta.total, 2);
    });

    test('supports a `user_id` parameter', async function(assert) {
      this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crateId: 'foo' });
      this.server.create('crate', { name: 'bar', _owner_users: [42] });
      this.server.create('version', { crateId: 'bar' });
      this.server.create('crate', { name: 'baz', _owner_users: [13] });
      this.server.create('version', { crateId: 'baz' });

      let response = await fetch('/api/v1/crates?user_id=42');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 1);
      assert.equal(responsePayload.crates[0].id, 'bar');
      assert.equal(responsePayload.meta.total, 1);
    });

    test('supports a `team_id` parameter', async function(assert) {
      this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crateId: 'foo' });
      this.server.create('crate', { name: 'bar', _owner_teams: [42] });
      this.server.create('version', { crateId: 'bar' });
      this.server.create('crate', { name: 'baz', _owner_teams: [13] });
      this.server.create('version', { crateId: 'baz' });

      let response = await fetch('/api/v1/crates?team_id=42');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 1);
      assert.equal(responsePayload.crates[0].id, 'bar');
      assert.equal(responsePayload.meta.total, 1);
    });

    test('supports a `team_id` parameter', async function(assert) {
      this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crateId: 'foo' });
      this.server.create('crate', { name: 'bar', _owner_teams: [42] });
      this.server.create('version', { crateId: 'bar' });
      this.server.create('crate', { name: 'baz', _owner_teams: [13] });
      this.server.create('version', { crateId: 'baz' });

      let response = await fetch('/api/v1/crates?team_id=42');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 1);
      assert.equal(responsePayload.crates[0].id, 'bar');
      assert.equal(responsePayload.meta.total, 1);
    });
  });

  module('GET /api/v1/crates/:id', function() {
    test('returns 404 for unknown crates', async function(assert) {
      let response = await fetch('/api/v1/crates/foo');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('returns a crate object for known crates', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crateId: 'rand', num: '1.0.0' });

      let response = await fetch('/api/v1/crates/rand');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
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
          max_version: '1.0.0',
          name: 'rand',
          newest_version: '1.0.0',
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
            dl_path: '/api/v1/crates/rand/1.0.0/download',
            downloads: 0,
            license: 'MIT/Apache-2.0',
            links: {
              authors: '/api/v1/crates/rand/1.0.0/authors',
              dependencies: '/api/v1/crates/rand/1.0.0/dependencies',
              version_downloads: '/api/v1/crates/rand/1.0.0/downloads',
            },
            num: '1.0.0',
            updated_at: '2017-02-24T12:34:56Z',
            yanked: false,
          },
        ],
      });
    });

    test('includes related versions', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crateId: 'rand', num: '1.0.0' });
      this.server.create('version', { crateId: 'rand', num: '1.1.0' });
      this.server.create('version', { crateId: 'rand', num: '1.2.0' });

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
            authors: '/api/v1/crates/rand/1.0.0/authors',
            dependencies: '/api/v1/crates/rand/1.0.0/dependencies',
            version_downloads: '/api/v1/crates/rand/1.0.0/downloads',
          },
          num: '1.0.0',
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
        },
        {
          id: '2',
          crate: 'rand',
          crate_size: 162963,
          created_at: '2010-06-16T21:30:45Z',
          dl_path: '/api/v1/crates/rand/1.1.0/download',
          downloads: 3702,
          license: 'MIT',
          links: {
            authors: '/api/v1/crates/rand/1.1.0/authors',
            dependencies: '/api/v1/crates/rand/1.1.0/dependencies',
            version_downloads: '/api/v1/crates/rand/1.1.0/downloads',
          },
          num: '1.1.0',
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
        },
        {
          id: '3',
          crate: 'rand',
          crate_size: 325926,
          created_at: '2010-06-16T21:30:45Z',
          dl_path: '/api/v1/crates/rand/1.2.0/download',
          downloads: 7404,
          license: 'Apache-2.0',
          links: {
            authors: '/api/v1/crates/rand/1.2.0/authors',
            dependencies: '/api/v1/crates/rand/1.2.0/dependencies',
            version_downloads: '/api/v1/crates/rand/1.2.0/downloads',
          },
          num: '1.2.0',
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
        },
      ]);
    });

    test('includes related categories', async function(assert) {
      this.server.create('category', { category: 'no-std' });
      this.server.create('category', { category: 'cli' });
      this.server.create('crate', { name: 'rand', categoryIds: ['no-std'] });
      this.server.create('version', { crateId: 'rand' });

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

    test('includes related keywords', async function(assert) {
      this.server.create('keyword', { keyword: 'no-std' });
      this.server.create('keyword', { keyword: 'cli' });
      this.server.create('crate', { name: 'rand', keywordIds: ['no-std'] });
      this.server.create('version', { crateId: 'rand' });

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

  module('GET /api/v1/crates/:id/versions', function() {
    test('returns 404 for unknown crates', async function(assert) {
      let response = await fetch('/api/v1/crates/foo/versions');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('empty case', async function(assert) {
      this.server.create('crate', { name: 'rand' });

      let response = await fetch('/api/v1/crates/rand/versions');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        versions: [],
      });
    });

    test('returns all versions belonging to the specified crate', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crateId: 'rand', num: '1.0.0' });
      this.server.create('version', { crateId: 'rand', num: '1.1.0' });
      this.server.create('version', { crateId: 'rand', num: '1.2.0' });

      let response = await fetch('/api/v1/crates/rand/versions');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
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
              authors: '/api/v1/crates/rand/1.0.0/authors',
              dependencies: '/api/v1/crates/rand/1.0.0/dependencies',
              version_downloads: '/api/v1/crates/rand/1.0.0/downloads',
            },
            num: '1.0.0',
            updated_at: '2017-02-24T12:34:56Z',
            yanked: false,
          },
          {
            id: '2',
            crate: 'rand',
            crate_size: 162963,
            created_at: '2010-06-16T21:30:45Z',
            dl_path: '/api/v1/crates/rand/1.1.0/download',
            downloads: 3702,
            license: 'MIT',
            links: {
              authors: '/api/v1/crates/rand/1.1.0/authors',
              dependencies: '/api/v1/crates/rand/1.1.0/dependencies',
              version_downloads: '/api/v1/crates/rand/1.1.0/downloads',
            },
            num: '1.1.0',
            updated_at: '2017-02-24T12:34:56Z',
            yanked: false,
          },
          {
            id: '3',
            crate: 'rand',
            crate_size: 325926,
            created_at: '2010-06-16T21:30:45Z',
            dl_path: '/api/v1/crates/rand/1.2.0/download',
            downloads: 7404,
            license: 'Apache-2.0',
            links: {
              authors: '/api/v1/crates/rand/1.2.0/authors',
              dependencies: '/api/v1/crates/rand/1.2.0/dependencies',
              version_downloads: '/api/v1/crates/rand/1.2.0/downloads',
            },
            num: '1.2.0',
            updated_at: '2017-02-24T12:34:56Z',
            yanked: false,
          },
        ],
      });
    });
  });

  module('GET /api/v1/crates/:id/:version/authors', function() {
    test('returns 404 for unknown crates', async function(assert) {
      let response = await fetch('/api/v1/crates/foo/1.0.0/authors');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('returns 200 for unknown versions', async function(assert) {
      this.server.create('crate', { name: 'rand' });

      let response = await fetch('/api/v1/crates/rand/1.0.0/authors');
      // we should probably return 404 for this, but the production API
      // currently doesn't do this either
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'crate `rand` does not have a version `1.0.0`' }] });
    });

    test('empty case', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crateId: 'rand', num: '1.0.0' });

      let response = await fetch('/api/v1/crates/rand/1.0.0/authors');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        meta: {
          names: [],
        },
        users: [],
      });
    });

    test('returns a list of authors belonging to the specified crate version', async function(assert) {
      let authors = ['John Doe <johnnydee@gmail.com>', 'The Rust Project Developers'];

      this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crateId: 'rand', num: '1.0.0', _authors: authors });

      let response = await fetch('/api/v1/crates/rand/1.0.0/authors');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        meta: {
          names: authors,
        },
        users: [],
      });
    });
  });

  module('GET /api/v1/crates/:id/:version/dependencies', function() {
    test('returns 404 for unknown crates', async function(assert) {
      let response = await fetch('/api/v1/crates/foo/1.0.0/dependencies');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('returns 200 for unknown versions', async function(assert) {
      this.server.create('crate', { name: 'rand' });

      let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
      // we should probably return 404 for this, but the production API
      // currently doesn't do this either
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'crate `rand` does not have a version `1.0.0`' }] });
    });

    test('empty case', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crateId: 'rand', num: '1.0.0' });

      let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        dependencies: [],
      });
    });

    test('returns a list of dependencies belonging to the specified crate version', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      let version = this.server.create('version', { crateId: 'rand', num: '1.0.0' });

      this.server.create('crate', { name: 'foo' });
      this.server.create('dependency', { crateId: 'foo', versionId: version.id });
      this.server.create('crate', { name: 'bar' });
      this.server.create('dependency', { crateId: 'bar', versionId: version.id });
      this.server.create('crate', { name: 'baz' });
      this.server.create('dependency', { crateId: 'baz', versionId: version.id });

      let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
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

  module('GET /api/v1/crates/:id/:version/downloads', function() {
    test('returns 404 for unknown crates', async function(assert) {
      let response = await fetch('/api/v1/crates/foo/1.0.0/downloads');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('returns 200 for unknown versions', async function(assert) {
      this.server.create('crate', { name: 'rand' });

      let response = await fetch('/api/v1/crates/rand/1.0.0/downloads');
      // we should probably return 404 for this, but the production API
      // currently doesn't do this either
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'crate `rand` does not have a version `1.0.0`' }] });
    });

    test('empty case', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crateId: 'rand', num: '1.0.0' });

      let response = await fetch('/api/v1/crates/rand/1.0.0/downloads');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        version_downloads: [],
      });
    });

    test('returns a list of version downloads belonging to the specified crate version', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      let version = this.server.create('version', { crateId: 'rand', num: '1.0.0' });
      this.server.create('version-download', { version, date: '2020-01-13' });
      this.server.create('version-download', { version, date: '2020-01-14' });
      this.server.create('version-download', { version, date: '2020-01-15' });

      let response = await fetch('/api/v1/crates/rand/1.0.0/downloads');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        version_downloads: [
          {
            date: '2020-01-13',
            downloads: 9380,
            version: '1',
          },
          {
            date: '2020-01-14',
            downloads: 16415,
            version: '1',
          },
          {
            date: '2020-01-15',
            downloads: 23450,
            version: '1',
          },
        ],
      });
    });
  });

  module('GET /api/v1/crates/:id/owner_user', function() {
    test('returns 404 for unknown crates', async function(assert) {
      let response = await fetch('/api/v1/crates/foo/owner_user');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('empty case', async function(assert) {
      this.server.create('crate', { name: 'rand' });

      let response = await fetch('/api/v1/crates/rand/owner_user');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        users: [],
      });
    });

    test('returns the list of users that own the specified crate', async function(assert) {
      let user = this.server.create('user', { name: 'John Doe' });
      this.server.create('crate', { name: 'rand', userOwners: [user] });

      let response = await fetch('/api/v1/crates/rand/owner_user');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        users: [
          {
            id: '1',
            avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
            kind: 'user',
            login: 'john-doe',
            name: 'John Doe',
            url: 'https://github.com/john-doe',
          },
        ],
      });
    });
  });

  module('GET /api/v1/crates/:id/owner_team', function() {
    test('returns 404 for unknown crates', async function(assert) {
      let response = await fetch('/api/v1/crates/foo/owner_team');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('empty case', async function(assert) {
      this.server.create('crate', { name: 'rand' });

      let response = await fetch('/api/v1/crates/rand/owner_team');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        teams: [],
      });
    });

    test('returns the list of teams that own the specified crate', async function(assert) {
      let team = this.server.create('team', { name: 'maintainers' });
      this.server.create('crate', { name: 'rand', teamOwners: [team] });

      let response = await fetch('/api/v1/crates/rand/owner_team');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        teams: [
          {
            id: '1',
            avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
            kind: 'team',
            login: 'github:rust-lang:maintainers',
            name: 'maintainers',
            url: 'https://github.com/rust-lang',
          },
        ],
      });
    });
  });

  module('GET /api/v1/crates/:id/reverse_dependencies', function() {
    test('returns 404 for unknown crates', async function(assert) {
      let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('empty case', async function(assert) {
      this.server.create('crate', { name: 'rand' });

      let response = await fetch('/api/v1/crates/rand/reverse_dependencies');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        dependencies: [],
        versions: [],
        meta: {
          total: 0,
        },
      });
    });

    test('returns a paginated list of crate versions depending to the specified crate', async function(assert) {
      this.server.create('crate', { name: 'foo' });

      this.server.create('dependency', {
        crateId: 'foo',
        versionId: this.server.create('version', {
          crate: this.server.create('crate', { name: 'bar' }),
        }).id,
      });

      this.server.create('dependency', {
        crateId: 'foo',
        versionId: this.server.create('version', {
          crate: this.server.create('crate', { name: 'baz' }),
        }).id,
      });

      let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
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
              authors: '/api/v1/crates/bar/1.0.0/authors',
              dependencies: '/api/v1/crates/bar/1.0.0/dependencies',
              version_downloads: '/api/v1/crates/bar/1.0.0/downloads',
            },
            num: '1.0.0',
            updated_at: '2017-02-24T12:34:56Z',
            yanked: false,
          },
          {
            id: '2',
            crate: 'baz',
            crate_size: 162963,
            created_at: '2010-06-16T21:30:45Z',
            dl_path: '/api/v1/crates/baz/1.0.1/download',
            downloads: 3702,
            license: 'MIT',
            links: {
              authors: '/api/v1/crates/baz/1.0.1/authors',
              dependencies: '/api/v1/crates/baz/1.0.1/dependencies',
              version_downloads: '/api/v1/crates/baz/1.0.1/downloads',
            },
            num: '1.0.1',
            updated_at: '2017-02-24T12:34:56Z',
            yanked: false,
          },
        ],
        meta: {
          total: 2,
        },
      });
    });

    test('never returns more than 10 results', async function(assert) {
      this.server.create('crate', { name: 'foo' });

      this.server.createList('dependency', 25, {
        crateId: 'foo',
        versionId: () =>
          this.server.create('version', {
            crate: () => this.server.create('crate', { name: 'bar' }),
          }).id,
      });

      let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.dependencies.length, 10);
      assert.equal(responsePayload.versions.length, 10);
      assert.equal(responsePayload.meta.total, 25);
    });

    test('supports `page` and `per_page` parameters', async function(assert) {
      this.server.create('crate', { name: 'foo' });

      let crates = this.server.createList('crate', 25, {
        name: i => `crate-${String(i + 1).padStart(2, '0')}`,
      });
      let versions = this.server.createList('version', crates.length, {
        crate: i => crates[i],
      });
      this.server.createList('dependency', versions.length, {
        crateId: 'foo',
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

  module('GET /api/v1/crates/:id/downloads', function() {
    test('returns 404 for unknown crates', async function(assert) {
      let response = await fetch('/api/v1/crates/foo/downloads');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('empty case', async function(assert) {
      this.server.create('crate', { name: 'rand' });

      let response = await fetch('/api/v1/crates/rand/downloads');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        version_downloads: [],
        meta: {
          extra_downloads: [],
        },
      });
    });

    test('returns a list of version downloads belonging to the specified crate version', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      let versions = this.server.createList('version', 2, { crateId: 'rand' });
      this.server.create('version-download', { version: versions[0], date: '2020-01-13' });
      this.server.create('version-download', { version: versions[1], date: '2020-01-14' });
      this.server.create('version-download', { version: versions[1], date: '2020-01-15' });

      let response = await fetch('/api/v1/crates/rand/downloads');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        version_downloads: [
          {
            date: '2020-01-13',
            downloads: 9380,
            version: '1',
          },
          {
            date: '2020-01-14',
            downloads: 16415,
            version: '2',
          },
          {
            date: '2020-01-15',
            downloads: 23450,
            version: '2',
          },
        ],
        meta: {
          extra_downloads: [],
        },
      });
    });
  });
});
