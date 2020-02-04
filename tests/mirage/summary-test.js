import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from '../helpers/setup-mirage';
import fetch from 'fetch';

module('Mirage | Summary', function(hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('GET /api/v1/summary', function() {
    test('empty case', async function(assert) {
      let response = await fetch('/api/v1/summary');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        just_updated: [],
        most_downloaded: [],
        most_recently_downloaded: [],
        new_crates: [],
        num_crates: 0,
        num_downloads: 0,
        popular_categories: [],
        popular_keywords: [],
      });
    });

    test('returns the data for the front page', async function(assert) {
      this.server.createList('category', 15);
      this.server.createList('keyword', 25);
      let crates = this.server.createList('crate', 20);
      this.server.createList('version', crates.length, { crate: i => crates[i] });

      let response = await fetch('/api/v1/summary');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();

      assert.equal(responsePayload.just_updated.length, 10);
      assert.deepEqual(responsePayload.just_updated[0], {
        id: 'crate-0',
        badges: [],
        categories: [],
        created_at: '2010-06-16T21:30:45Z',
        description: 'This is the description for the crate called "crate-0"',
        documentation: null,
        downloads: 0,
        homepage: null,
        keywords: [],
        links: {
          owner_team: '/api/v1/crates/crate-0/owner_team',
          owner_user: '/api/v1/crates/crate-0/owner_user',
          reverse_dependencies: '/api/v1/crates/crate-0/reverse_dependencies',
          version_downloads: '/api/v1/crates/crate-0/downloads',
          versions: '/api/v1/crates/crate-0/versions',
        },
        max_version: '1.0.0',
        name: 'crate-0',
        newest_version: '1.0.0',
        repository: null,
        updated_at: '2017-02-24T12:34:56Z',
        versions: null,
      });

      assert.equal(responsePayload.most_downloaded.length, 10);
      assert.deepEqual(responsePayload.most_downloaded[0], {
        id: 'crate-4',
        badges: [],
        categories: [],
        created_at: '2010-06-16T21:30:45Z',
        description: 'This is the description for the crate called "crate-4"',
        documentation: null,
        downloads: 148140,
        homepage: null,
        keywords: [],
        links: {
          owner_team: '/api/v1/crates/crate-4/owner_team',
          owner_user: '/api/v1/crates/crate-4/owner_user',
          reverse_dependencies: '/api/v1/crates/crate-4/reverse_dependencies',
          version_downloads: '/api/v1/crates/crate-4/downloads',
          versions: '/api/v1/crates/crate-4/versions',
        },
        max_version: '1.0.4',
        name: 'crate-4',
        newest_version: '1.0.4',
        repository: null,
        updated_at: '2017-02-24T12:34:56Z',
        versions: null,
      });

      assert.equal(responsePayload.most_recently_downloaded.length, 10);
      assert.deepEqual(responsePayload.most_recently_downloaded[0], {
        id: 'crate-0',
        badges: [],
        categories: [],
        created_at: '2010-06-16T21:30:45Z',
        description: 'This is the description for the crate called "crate-0"',
        documentation: null,
        downloads: 0,
        homepage: null,
        keywords: [],
        links: {
          owner_team: '/api/v1/crates/crate-0/owner_team',
          owner_user: '/api/v1/crates/crate-0/owner_user',
          reverse_dependencies: '/api/v1/crates/crate-0/reverse_dependencies',
          version_downloads: '/api/v1/crates/crate-0/downloads',
          versions: '/api/v1/crates/crate-0/versions',
        },
        max_version: '1.0.0',
        name: 'crate-0',
        newest_version: '1.0.0',
        repository: null,
        updated_at: '2017-02-24T12:34:56Z',
        versions: null,
      });

      assert.equal(responsePayload.new_crates.length, 10);
      assert.deepEqual(responsePayload.new_crates[0], {
        id: 'crate-0',
        badges: [],
        categories: [],
        created_at: '2010-06-16T21:30:45Z',
        description: 'This is the description for the crate called "crate-0"',
        documentation: null,
        downloads: 0,
        homepage: null,
        keywords: [],
        links: {
          owner_team: '/api/v1/crates/crate-0/owner_team',
          owner_user: '/api/v1/crates/crate-0/owner_user',
          reverse_dependencies: '/api/v1/crates/crate-0/reverse_dependencies',
          version_downloads: '/api/v1/crates/crate-0/downloads',
          versions: '/api/v1/crates/crate-0/versions',
        },
        max_version: '1.0.0',
        name: 'crate-0',
        newest_version: '1.0.0',
        repository: null,
        updated_at: '2017-02-24T12:34:56Z',
        versions: null,
      });

      assert.equal(responsePayload.num_crates, 20);
      assert.equal(responsePayload.num_downloads, 1419675);

      assert.equal(responsePayload.popular_categories.length, 10);
      assert.deepEqual(responsePayload.popular_categories[0], {
        id: 'category-0',
        category: 'Category 0',
        crates_cnt: 0,
        created_at: '2010-06-16T21:30:45Z',
        description: 'This is the description for the category called "Category 0"',
        slug: 'category-0',
      });

      assert.equal(responsePayload.popular_keywords.length, 10);
      assert.deepEqual(responsePayload.popular_keywords[0], {
        id: 'keyword-1',
        crates_cnt: 0,
        keyword: 'keyword-1',
      });
    });
  });
});
