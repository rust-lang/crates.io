import { assert, test } from 'vitest';

import { db } from '../index.js';

test('empty case', async function () {
  let response = await fetch('/api/v1/summary');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
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

test('returns the data for the front page', async function () {
  Array.from({ length: 15 }, () => db.category.create());
  Array.from({ length: 25 }, () => db.keyword.create());
  let crates = Array.from({ length: 20 }, () => db.crate.create());
  crates.forEach(crate => db.version.create({ crate }));

  let response = await fetch('/api/v1/summary');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();

  assert.strictEqual(responsePayload.just_updated.length, 10);
  assert.deepEqual(responsePayload.just_updated[0], {
    id: 'crate-1',
    badges: [],
    categories: null,
    created_at: '2010-06-16T21:30:45Z',
    default_version: '1.0.0',
    description: 'This is the description for the crate called "crate-1"',
    documentation: null,
    downloads: 37_035,
    homepage: null,
    keywords: null,
    links: {
      owner_team: '/api/v1/crates/crate-1/owner_team',
      owner_user: '/api/v1/crates/crate-1/owner_user',
      reverse_dependencies: '/api/v1/crates/crate-1/reverse_dependencies',
      version_downloads: '/api/v1/crates/crate-1/downloads',
      versions: '/api/v1/crates/crate-1/versions',
    },
    max_version: '1.0.0',
    max_stable_version: '1.0.0',
    name: 'crate-1',
    newest_version: '1.0.0',
    recent_downloads: 321,
    repository: null,
    updated_at: '2017-02-24T12:34:56Z',
    versions: null,
    yanked: false,
  });

  assert.strictEqual(responsePayload.most_downloaded.length, 10);
  assert.deepEqual(responsePayload.most_downloaded[0], {
    id: 'crate-4',
    badges: [],
    categories: null,
    created_at: '2010-06-16T21:30:45Z',
    default_version: '1.0.3',
    description: 'This is the description for the crate called "crate-4"',
    documentation: null,
    downloads: 148_140,
    homepage: null,
    keywords: null,
    links: {
      owner_team: '/api/v1/crates/crate-4/owner_team',
      owner_user: '/api/v1/crates/crate-4/owner_user',
      reverse_dependencies: '/api/v1/crates/crate-4/reverse_dependencies',
      version_downloads: '/api/v1/crates/crate-4/downloads',
      versions: '/api/v1/crates/crate-4/versions',
    },
    max_version: '1.0.3',
    max_stable_version: '1.0.3',
    name: 'crate-4',
    newest_version: '1.0.3',
    repository: null,
    recent_downloads: 963,
    updated_at: '2017-02-24T12:34:56Z',
    versions: null,
    yanked: false,
  });

  assert.strictEqual(responsePayload.most_recently_downloaded.length, 10);
  assert.deepEqual(responsePayload.most_recently_downloaded[0], {
    id: 'crate-11',
    badges: [],
    categories: null,
    created_at: '2010-06-16T21:30:45Z',
    default_version: '1.0.10',
    description: 'This is the description for the crate called "crate-11"',
    documentation: null,
    downloads: 86_415,
    homepage: null,
    keywords: null,
    links: {
      owner_team: '/api/v1/crates/crate-11/owner_team',
      owner_user: '/api/v1/crates/crate-11/owner_user',
      reverse_dependencies: '/api/v1/crates/crate-11/reverse_dependencies',
      version_downloads: '/api/v1/crates/crate-11/downloads',
      versions: '/api/v1/crates/crate-11/versions',
    },
    max_version: '1.0.10',
    max_stable_version: '1.0.10',
    name: 'crate-11',
    newest_version: '1.0.10',
    repository: null,
    recent_downloads: 3852,
    updated_at: '2017-02-24T12:34:56Z',
    versions: null,
    yanked: false,
  });

  assert.strictEqual(responsePayload.new_crates.length, 10);
  assert.deepEqual(responsePayload.new_crates[0], {
    id: 'crate-20',
    badges: [],
    categories: null,
    created_at: '2010-06-16T21:30:45Z',
    default_version: '1.0.19',
    description: 'This is the description for the crate called "crate-20"',
    documentation: null,
    downloads: 98_760,
    homepage: null,
    keywords: null,
    links: {
      owner_team: '/api/v1/crates/crate-20/owner_team',
      owner_user: '/api/v1/crates/crate-20/owner_user',
      reverse_dependencies: '/api/v1/crates/crate-20/reverse_dependencies',
      version_downloads: '/api/v1/crates/crate-20/downloads',
      versions: '/api/v1/crates/crate-20/versions',
    },
    max_version: '1.0.19',
    max_stable_version: '1.0.19',
    name: 'crate-20',
    newest_version: '1.0.19',
    repository: null,
    recent_downloads: 1605,
    updated_at: '2017-02-24T12:34:56Z',
    versions: null,
    yanked: false,
  });

  assert.strictEqual(responsePayload.num_crates, 20);
  assert.strictEqual(responsePayload.num_downloads, 1_518_435);

  assert.strictEqual(responsePayload.popular_categories.length, 10);
  assert.deepEqual(responsePayload.popular_categories[0], {
    id: 'category-1',
    category: 'Category 1',
    crates_cnt: 0,
    created_at: '2010-06-16T21:30:45Z',
    description: 'This is the description for the category called "Category 1"',
    slug: 'category-1',
  });

  assert.strictEqual(responsePayload.popular_keywords.length, 10);
  assert.deepEqual(responsePayload.popular_keywords[0], {
    id: 'keyword-1',
    crates_cnt: 0,
    keyword: 'keyword-1',
  });
});
