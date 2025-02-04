import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0-beta.1');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns 404 for unknown versions', async function () {
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '1.0.0-alpha.1' });
  let response = await fetch('/api/v1/crates/rand/1.0.0-beta.1');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'crate `rand` does not have a version `1.0.0-beta.1`' }],
  });
});

test('returns a version object for known version', async function () {
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '1.0.0-beta.1' });

  let response = await fetch('/api/v1/crates/rand/1.0.0-beta.1');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    version: {
      crate: 'rand',
      crate_size: 162_963,
      created_at: '2010-06-16T21:30:45Z',
      dl_path: '/api/v1/crates/rand/1.0.0-beta.1/download',
      downloads: 3702,
      features: {},
      id: 1,
      license: 'MIT',
      links: {
        dependencies: '/api/v1/crates/rand/1.0.0-beta.1/dependencies',
        version_downloads: '/api/v1/crates/rand/1.0.0-beta.1/downloads',
      },
      num: '1.0.0-beta.1',
      published_by: null,
      readme_path: '/api/v1/crates/rand/1.0.0-beta.1/readme',
      rust_version: null,
      updated_at: '2017-02-24T12:34:56Z',
      yank_message: null,
      yanked: false,
    },
  });
});
