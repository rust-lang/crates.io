import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns a crate object for known crates', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0-beta.1' });

  let response = await fetch('/api/v1/crates/rand');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    categories: [],
    crate: {
      badges: [],
      categories: [],
      created_at: '2010-06-16T21:30:45Z',
      default_version: '1.0.0-beta.1',
      description: 'This is the description for the crate called "rand"',
      documentation: null,
      downloads: 37_035,
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
      num_versions: 1,
      repository: null,
      recent_downloads: 321,
      trustpub_only: false,
      updated_at: '2017-02-24T12:34:56Z',
      versions: [1],
      yanked: false,
    },
    keywords: [],
    versions: [
      {
        id: 1,
        crate: 'rand',
        crate_size: 162_963,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/rand/1.0.0-beta.1/download',
        downloads: 3702,
        features: {},
        license: 'MIT',
        linecounts: {
          languages: {
            JavaScript: {
              code_lines: 325,
              comment_lines: 80,
              files: 8,
            },
            TypeScript: {
              code_lines: 195,
              comment_lines: 10,
              files: 2,
            },
          },
          total_code_lines: 520,
          total_comment_lines: 90,
        },
        links: {
          dependencies: '/api/v1/crates/rand/1.0.0-beta.1/dependencies',
          version_downloads: '/api/v1/crates/rand/1.0.0-beta.1/downloads',
        },
        num: '1.0.0-beta.1',
        published_by: null,
        readme_path: '/api/v1/crates/rand/1.0.0-beta.1/readme',
        rust_version: null,
        trustpub_data: null,
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
        yank_message: null,
      },
    ],
  });
});

test('works for non-canonical names', async function () {
  let crate = await db.crate.create({ name: 'foo-bar' });
  await db.version.create({ crate, num: '1.0.0-beta.1' });

  let response = await fetch('/api/v1/crates/foo_bar');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    categories: [],
    crate: {
      badges: [],
      categories: [],
      created_at: '2010-06-16T21:30:45Z',
      default_version: '1.0.0-beta.1',
      description: 'This is the description for the crate called "foo-bar"',
      documentation: null,
      downloads: 37_035,
      homepage: null,
      id: 'foo-bar',
      keywords: [],
      links: {
        owner_team: '/api/v1/crates/foo-bar/owner_team',
        owner_user: '/api/v1/crates/foo-bar/owner_user',
        reverse_dependencies: '/api/v1/crates/foo-bar/reverse_dependencies',
        version_downloads: '/api/v1/crates/foo-bar/downloads',
        versions: '/api/v1/crates/foo-bar/versions',
      },
      max_version: '1.0.0-beta.1',
      max_stable_version: null,
      name: 'foo-bar',
      newest_version: '1.0.0-beta.1',
      num_versions: 1,
      repository: null,
      recent_downloads: 321,
      trustpub_only: false,
      updated_at: '2017-02-24T12:34:56Z',
      versions: [1],
      yanked: false,
    },
    keywords: [],
    versions: [
      {
        id: 1,
        crate: 'foo-bar',
        crate_size: 162_963,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/foo-bar/1.0.0-beta.1/download',
        downloads: 3702,
        features: {},
        license: 'MIT',
        linecounts: {
          languages: {
            JavaScript: {
              code_lines: 325,
              comment_lines: 80,
              files: 8,
            },
            TypeScript: {
              code_lines: 195,
              comment_lines: 10,
              files: 2,
            },
          },
          total_code_lines: 520,
          total_comment_lines: 90,
        },
        links: {
          dependencies: '/api/v1/crates/foo-bar/1.0.0-beta.1/dependencies',
          version_downloads: '/api/v1/crates/foo-bar/1.0.0-beta.1/downloads',
        },
        num: '1.0.0-beta.1',
        published_by: null,
        readme_path: '/api/v1/crates/foo-bar/1.0.0-beta.1/readme',
        rust_version: null,
        trustpub_data: null,
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
        yank_message: null,
      },
    ],
  });
});

test('includes related versions', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });
  await db.version.create({ crate, num: '1.1.0' });
  await db.version.create({ crate, num: '1.2.0' });

  let response = await fetch('/api/v1/crates/rand');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.deepEqual(responsePayload.crate.versions, [1, 2, 3]);
  assert.deepEqual(responsePayload.versions, [
    {
      id: 3,
      crate: 'rand',
      crate_size: 488_889,
      created_at: '2010-06-16T21:30:45Z',
      dl_path: '/api/v1/crates/rand/1.2.0/download',
      downloads: 11_106,
      features: {},
      license: 'MIT/Apache-2.0',
      linecounts: {
        languages: {
          Python: {
            code_lines: 421,
            comment_lines: 64,
            files: 8,
          },
        },
        total_code_lines: 421,
        total_comment_lines: 64,
      },
      links: {
        dependencies: '/api/v1/crates/rand/1.2.0/dependencies',
        version_downloads: '/api/v1/crates/rand/1.2.0/downloads',
      },
      num: '1.2.0',
      published_by: null,
      readme_path: '/api/v1/crates/rand/1.2.0/readme',
      rust_version: null,
      trustpub_data: null,
      updated_at: '2017-02-24T12:34:56Z',
      yanked: false,
      yank_message: null,
    },
    {
      id: 2,
      crate: 'rand',
      crate_size: 325_926,
      created_at: '2010-06-16T21:30:45Z',
      dl_path: '/api/v1/crates/rand/1.1.0/download',
      downloads: 7404,
      features: {},
      license: 'Apache-2.0',
      linecounts: {
        languages: {
          CSS: {
            code_lines: 503,
            comment_lines: 42,
            files: 2,
          },
          Python: {
            code_lines: 284,
            comment_lines: 91,
            files: 3,
          },
          TypeScript: {
            code_lines: 332,
            comment_lines: 83,
            files: 7,
          },
        },
        total_code_lines: 1119,
        total_comment_lines: 216,
      },
      links: {
        dependencies: '/api/v1/crates/rand/1.1.0/dependencies',
        version_downloads: '/api/v1/crates/rand/1.1.0/downloads',
      },
      num: '1.1.0',
      published_by: null,
      readme_path: '/api/v1/crates/rand/1.1.0/readme',
      rust_version: null,
      trustpub_data: null,
      updated_at: '2017-02-24T12:34:56Z',
      yanked: false,
      yank_message: null,
    },
    {
      id: 1,
      crate: 'rand',
      crate_size: 162_963,
      created_at: '2010-06-16T21:30:45Z',
      dl_path: '/api/v1/crates/rand/1.0.0/download',
      downloads: 3702,
      features: {},
      license: 'MIT',
      linecounts: {
        languages: {
          JavaScript: {
            code_lines: 325,
            comment_lines: 80,
            files: 8,
          },
          TypeScript: {
            code_lines: 195,
            comment_lines: 10,
            files: 2,
          },
        },
        total_code_lines: 520,
        total_comment_lines: 90,
      },
      links: {
        dependencies: '/api/v1/crates/rand/1.0.0/dependencies',
        version_downloads: '/api/v1/crates/rand/1.0.0/downloads',
      },
      num: '1.0.0',
      published_by: null,
      readme_path: '/api/v1/crates/rand/1.0.0/readme',
      rust_version: null,
      trustpub_data: null,
      updated_at: '2017-02-24T12:34:56Z',
      yanked: false,
      yank_message: null,
    },
  ]);
});

test('includes related categories', async function () {
  let noStd = await db.category.create({ category: 'no-std' });
  await db.category.create({ category: 'cli' });
  let crate = await db.crate.create({ name: 'rand', categories: [noStd] });
  await db.version.create({ crate });

  let response = await fetch('/api/v1/crates/rand');
  assert.strictEqual(response.status, 200);

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

test('includes related keywords', async function () {
  let noStd = await db.keyword.create({ keyword: 'no-std' });
  await db.keyword.create({ keyword: 'cli' });
  let crate = await db.crate.create({ name: 'rand', keywords: [noStd] });
  await db.version.create({ crate });

  let response = await fetch('/api/v1/crates/rand');
  assert.strictEqual(response.status, 200);

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

test('without versions included', async function () {
  await db.category.create({ category: 'no-std' });
  await db.category.create({ category: 'cli' });
  await db.keyword.create({ keyword: 'no-std' });
  await db.keyword.create({ keyword: 'cli' });
  let crate = await db.crate.create({ name: 'rand', categoryIds: ['no-std'], keywordIds: ['no-std'] });
  await db.version.create({ crate, num: '1.0.0' });
  await db.version.create({ crate, num: '1.1.0' });
  await db.version.create({ crate, num: '1.2.0' });

  let req = await fetch('/api/v1/crates/rand');
  let expected = await req.json();

  let response = await fetch('/api/v1/crates/rand?include=keywords,categories');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.deepEqual(responsePayload, {
    ...expected,
    crate: {
      ...expected.crate,
      max_version: '0.0.0',
      newest_version: '0.0.0',
      max_stable_version: null,
      versions: null,
    },
    versions: null,
  });
});

test('includes default_version', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });
  await db.version.create({ crate, num: '1.1.0' });
  await db.version.create({ crate, num: '1.2.0' });

  let req = await fetch('/api/v1/crates/rand');
  let expected = await req.json();

  let response = await fetch('/api/v1/crates/rand?include=default_version');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  let default_version = expected.versions.find(it => it.num === responsePayload.crate.default_version);
  assert.deepEqual(responsePayload, {
    ...expected,
    crate: {
      ...expected.crate,
      categories: null,
      keywords: null,
      max_version: '0.0.0',
      newest_version: '0.0.0',
      max_stable_version: null,
      versions: null,
    },
    categories: null,
    keywords: null,
    versions: [default_version],
  });

  let resp_both = await fetch('/api/v1/crates/rand?include=versions,default_version');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await resp_both.json(), {
    ...expected,
    crate: {
      ...expected.crate,
      categories: null,
      keywords: null,
    },
    categories: null,
    keywords: null,
  });
});
