import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/downloads');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('empty case', async function () {
  await db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/downloads');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    version_downloads: [],
    meta: {
      extra_downloads: [],
    },
  });
});

test('returns a list of version downloads belonging to the specified crate version', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  let versions = await Promise.all(Array.from({ length: 2 }, () => db.version.create({ crate })));
  await db.versionDownload.create({ version: versions[0], date: '2020-01-13' });
  await db.versionDownload.create({ version: versions[1], date: '2020-01-14' });
  await db.versionDownload.create({ version: versions[1], date: '2020-01-15' });

  let response = await fetch('/api/v1/crates/rand/downloads');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    version_downloads: [
      {
        date: '2020-01-13',
        downloads: 7035,
        version: 1,
      },
      {
        date: '2020-01-14',
        downloads: 14_070,
        version: 2,
      },
      {
        date: '2020-01-15',
        downloads: 21_105,
        version: 2,
      },
    ],
    meta: {
      extra_downloads: [],
    },
  });
});

test('includes related versions', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  let versions = await Promise.all(Array.from({ length: 2 }, () => db.version.create({ crate })));
  await db.versionDownload.create({ version: versions[0], date: '2020-01-13' });
  await db.versionDownload.create({ version: versions[1], date: '2020-01-14' });
  await db.versionDownload.create({ version: versions[1], date: '2020-01-15' });

  let response = await fetch('/api/v1/crates/rand/downloads?include=versions');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    version_downloads: [
      {
        date: '2020-01-13',
        downloads: 7035,
        version: 1,
      },
      {
        date: '2020-01-14',
        downloads: 14_070,
        version: 2,
      },
      {
        date: '2020-01-15',
        downloads: 21_105,
        version: 2,
      },
    ],
    versions: [
      {
        crate: 'rand',
        crate_size: 162_963,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/rand/1.0.0/download',
        downloads: 3702,
        features: {},
        id: 1,
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
        yank_message: null,
        yanked: false,
      },
      {
        crate: 'rand',
        crate_size: 325_926,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/rand/1.0.1/download',
        downloads: 7404,
        features: {},
        id: 2,
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
          dependencies: '/api/v1/crates/rand/1.0.1/dependencies',
          version_downloads: '/api/v1/crates/rand/1.0.1/downloads',
        },
        num: '1.0.1',
        published_by: null,
        readme_path: '/api/v1/crates/rand/1.0.1/readme',
        rust_version: null,
        trustpub_data: null,
        updated_at: '2017-02-24T12:34:56Z',
        yank_message: null,
        yanked: false,
      },
    ],
    meta: {
      extra_downloads: [],
    },
  });
});
