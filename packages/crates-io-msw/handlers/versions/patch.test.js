import { assert, test } from 'vitest';

import { db } from '../../index.js';

const YANK_BODY = JSON.stringify({
  version: {
    yanked: true,
    yank_message: 'some reason',
  },
});

const UNYANK_BODY = JSON.stringify({
  version: {
    yanked: false,
  },
});

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns 404 for unknown versions', async function () {
  db.crate.create({ name: 'foo' });

  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('yanks the version', async function () {
  let crate = db.crate.create({ name: 'foo' });
  let version = db.version.create({ crate, num: '1.0.0', yanked: false });
  assert.strictEqual(version.yanked, false);
  assert.strictEqual(version.yank_message, null);

  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    version: {
      crate: 'foo',
      crate_size: 162_963,
      created_at: '2010-06-16T21:30:45Z',
      dl_path: '/api/v1/crates/foo/1.0.0/download',
      downloads: 3702,
      features: {},
      id: 1,
      license: 'MIT',
      links: {
        dependencies: '/api/v1/crates/foo/1.0.0/dependencies',
        version_downloads: '/api/v1/crates/foo/1.0.0/downloads',
      },
      num: '1.0.0',
      published_by: null,
      readme_path: '/api/v1/crates/foo/1.0.0/readme',
      rust_version: null,
      updated_at: '2017-02-24T12:34:56Z',
      yank_message: 'some reason',
      yanked: true,
    },
  });

  version = db.version.findFirst({ where: { id: { equals: version.id } } });
  assert.strictEqual(version.yanked, true);
  assert.strictEqual(version.yank_message, 'some reason');

  response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: UNYANK_BODY });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    version: {
      crate: 'foo',
      crate_size: 162_963,
      created_at: '2010-06-16T21:30:45Z',
      dl_path: '/api/v1/crates/foo/1.0.0/download',
      downloads: 3702,
      features: {},
      id: 1,
      license: 'MIT',
      links: {
        dependencies: '/api/v1/crates/foo/1.0.0/dependencies',
        version_downloads: '/api/v1/crates/foo/1.0.0/downloads',
      },
      num: '1.0.0',
      published_by: null,
      readme_path: '/api/v1/crates/foo/1.0.0/readme',
      rust_version: null,
      updated_at: '2017-02-24T12:34:56Z',
      yank_message: null,
      yanked: false,
    },
  });

  version = db.version.findFirst({ where: { id: { equals: version.id } } });
  assert.strictEqual(version.yanked, false);
  assert.strictEqual(version.yank_message, null);
});
