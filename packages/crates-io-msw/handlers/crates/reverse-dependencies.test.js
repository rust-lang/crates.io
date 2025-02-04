import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('empty case', async function () {
  db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/reverse_dependencies');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    dependencies: [],
    versions: [],
    meta: {
      total: 0,
    },
  });
});

test('returns a paginated list of crate versions depending to the specified crate', async function () {
  let crate = db.crate.create({ name: 'foo' });

  db.dependency.create({
    crate,
    version: db.version.create({
      crate: db.crate.create({ name: 'bar' }),
    }),
  });

  db.dependency.create({
    crate,
    version: db.version.create({
      crate: db.crate.create({ name: 'baz' }),
    }),
  });

  let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    dependencies: [
      {
        id: 2,
        crate_id: 'foo',
        default_features: false,
        features: [],
        kind: 'normal',
        optional: true,
        req: '0.3.7',
        target: null,
        version_id: 2,
      },
      {
        id: 1,
        crate_id: 'foo',
        default_features: false,
        features: [],
        kind: 'normal',
        optional: true,
        req: '^2.1.3',
        target: null,
        version_id: 1,
      },
    ],
    versions: [
      {
        id: 2,
        crate: 'baz',
        crate_size: 325_926,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/baz/1.0.1/download',
        downloads: 7404,
        features: {},
        license: 'Apache-2.0',
        links: {
          dependencies: '/api/v1/crates/baz/1.0.1/dependencies',
          version_downloads: '/api/v1/crates/baz/1.0.1/downloads',
        },
        num: '1.0.1',
        published_by: null,
        readme_path: '/api/v1/crates/baz/1.0.1/readme',
        rust_version: null,
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
        yank_message: null,
      },
      {
        id: 1,
        crate: 'bar',
        crate_size: 162_963,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/bar/1.0.0/download',
        downloads: 3702,
        features: {},
        license: 'MIT',
        links: {
          dependencies: '/api/v1/crates/bar/1.0.0/dependencies',
          version_downloads: '/api/v1/crates/bar/1.0.0/downloads',
        },
        num: '1.0.0',
        published_by: null,
        readme_path: '/api/v1/crates/bar/1.0.0/readme',
        rust_version: null,
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
        yank_message: null,
      },
    ],
    meta: {
      total: 2,
    },
  });
});

test('never returns more than 10 results', async function () {
  let crate = db.crate.create({ name: 'foo' });

  Array.from({ length: 25 }, () =>
    db.dependency.create({
      crate,
      version: db.version.create({
        crate: db.crate.create({ name: 'bar' }),
      }),
    }),
  );

  let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.dependencies.length, 10);
  assert.strictEqual(responsePayload.versions.length, 10);
  assert.strictEqual(responsePayload.meta.total, 25);
});

test('supports `page` and `per_page` parameters', async function () {
  let crate = db.crate.create({ name: 'foo' });

  let crates = Array.from({ length: 25 }, (_, i) =>
    db.crate.create({ name: `crate-${String(i + 1).padStart(2, '0')}` }),
  );
  let versions = crates.map(crate => db.version.create({ crate }));
  versions.forEach(version => db.dependency.create({ crate, version }));

  let response = await fetch('/api/v1/crates/foo/reverse_dependencies?page=2&per_page=5');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.dependencies.length, 5);
  assert.deepEqual(
    responsePayload.versions.map(it => it.crate),
    ['crate-24', 'crate-02', 'crate-15', 'crate-06', 'crate-19'],
  );
  assert.strictEqual(responsePayload.meta.total, 25);
});
