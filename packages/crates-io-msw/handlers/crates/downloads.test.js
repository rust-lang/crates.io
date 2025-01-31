import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/downloads');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('empty case', async function () {
  db.crate.create({ name: 'rand' });

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
  let crate = db.crate.create({ name: 'rand' });
  let versions = Array.from({ length: 2 }, () => db.version.create({ crate }));
  db.versionDownload.create({ version: versions[0], date: '2020-01-13' });
  db.versionDownload.create({ version: versions[1], date: '2020-01-14' });
  db.versionDownload.create({ version: versions[1], date: '2020-01-15' });

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
