import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from '../helpers/setup-mirage';
import fetch from 'fetch';

module('Mirage | Keywords', function(hooks) {
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
            versions: [],
          },
        ],
        meta: {
          total: 1,
        },
      });
    });

    test('never returns more than 10 results', async function(assert) {
      this.server.createList('crate', 25);

      let response = await fetch('/api/v1/crates');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 10);
      assert.equal(responsePayload.meta.total, 25);
    });

    test('supports `page` and `per_page` parameters', async function(assert) {
      this.server.createList('crate', 25, {
        name: i => `crate-${String(i + 1).padStart(2, '0')}`,
      });

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
      this.server.create('crate', { name: 'bar' });
      this.server.create('crate', { name: 'BAZ' });

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
      this.server.create('crate', { name: '00123' });
      this.server.create('crate', { name: '87654' });

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
      this.server.create('crate', { name: 'bar', _owner_users: [42] });
      this.server.create('crate', { name: 'baz', _owner_users: [13] });

      let response = await fetch('/api/v1/crates?user_id=42');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 1);
      assert.equal(responsePayload.crates[0].id, 'bar');
      assert.equal(responsePayload.meta.total, 1);
    });

    test('supports a `team_id` parameter', async function(assert) {
      this.server.create('crate', { name: 'foo' });
      this.server.create('crate', { name: 'bar', _owner_teams: [42] });
      this.server.create('crate', { name: 'baz', _owner_teams: [13] });

      let response = await fetch('/api/v1/crates?team_id=42');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.crates.length, 1);
      assert.equal(responsePayload.crates[0].id, 'bar');
      assert.equal(responsePayload.meta.total, 1);
    });

    test('supports a `team_id` parameter', async function(assert) {
      this.server.create('crate', { name: 'foo' });
      this.server.create('crate', { name: 'bar', _owner_teams: [42] });
      this.server.create('crate', { name: 'baz', _owner_teams: [13] });

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
          versions: [],
        },
        keywords: [],
        versions: [],
      });
    });

    test('includes related versions', async function(assert) {
      this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crate: 'rand', num: '1.0.0' });
      this.server.create('version', { crate: 'rand', num: '1.1.0' });
      this.server.create('version', { crate: 'rand', num: '1.2.0' });

      let response = await fetch('/api/v1/crates/rand');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
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
      this.server.create('crate', { name: 'rand', categories: ['no-std'] });

      let response = await fetch('/api/v1/crates/rand');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload.categories, [
        {
          id: 'no-std',
          category: 'no-std',
          crates_cnt: 0,
          created_at: '2010-06-16T21:30:45Z',
          description: 'This is the description for the category called "no-std"',
          slug: 'no-std',
        },
      ]);
    });

    test('includes related keywords', async function(assert) {
      this.server.create('keyword', { keyword: 'no-std' });
      this.server.create('keyword', { keyword: 'cli' });
      this.server.create('crate', { name: 'rand', keywords: ['no-std'] });

      let response = await fetch('/api/v1/crates/rand');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload.keywords, [
        {
          crates_cnt: 0,
          id: 'no-std',
          keyword: 'no-std',
        },
      ]);
    });
  });
});
