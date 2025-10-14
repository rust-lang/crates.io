import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 for unauthenticated user', async function () {
  let response = await fetch('/api/v1/me/updates');
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns latest versions of followed crates', async function () {
  let foo = await db.crate.create({ name: 'foo' });
  await db.version.create({ crate: foo, num: '1.2.3' });

  let bar = await db.crate.create({ name: 'bar' });
  await db.version.create({ crate: bar, num: '0.8.6' });

  let user = await db.user.create({ followedCrates: [foo] });
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/me/updates');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    versions: [
      {
        id: 1,
        crate: 'foo',
        crate_size: 162_963,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/foo/1.2.3/download',
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
          dependencies: '/api/v1/crates/foo/1.2.3/dependencies',
          version_downloads: '/api/v1/crates/foo/1.2.3/downloads',
        },
        num: '1.2.3',
        published_by: null,
        readme_path: '/api/v1/crates/foo/1.2.3/readme',
        rust_version: null,
        trustpub_data: null,
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
        yank_message: null,
      },
    ],
    meta: {
      more: false,
    },
  });
});

test('empty case', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/me/updates');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    versions: [],
    meta: { more: false },
  });
});

test('supports pagination', async function () {
  let crate = await db.crate.create({ name: 'foo' });
  await Promise.all(Array.from({ length: 25 }, () => db.version.create({ crate })));

  let user = await db.user.create({ followedCrates: [crate] });
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/me/updates?page=2');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.versions.length, 10);
  assert.deepEqual(
    responsePayload.versions.map(it => it.id),
    [15, 14, 13, 12, 11, 10, 9, 8, 7, 6],
  );
  assert.deepEqual(responsePayload.meta, { more: true });
});
