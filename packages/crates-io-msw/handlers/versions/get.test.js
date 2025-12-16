import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0-beta.1');
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({ errors: [{ detail: 'Not Found' }] });
});

test('returns 404 for unknown versions', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0-alpha.1' });
  let response = await fetch('/api/v1/crates/rand/1.0.0-beta.1');
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'crate `rand` does not have a version `1.0.0-beta.1`' }],
  });
});

test('returns a version object for known version', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0-beta.1' });

  let response = await fetch('/api/v1/crates/rand/1.0.0-beta.1');
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
    version: {
      crate: 'rand',
      crate_size: 162_963,
      created_at: '2010-06-16T21:30:45Z',
      dl_path: '/api/v1/crates/rand/1.0.0-beta.1/download',
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
        dependencies: '/api/v1/crates/rand/1.0.0-beta.1/dependencies',
        version_downloads: '/api/v1/crates/rand/1.0.0-beta.1/downloads',
      },
      num: '1.0.0-beta.1',
      published_by: null,
      readme_path: '/api/v1/crates/rand/1.0.0-beta.1/readme',
      rust_version: null,
      trustpub_data: null,
      updated_at: '2017-02-24T12:34:56Z',
      yank_message: null,
      yanked: false,
    },
  });
});
