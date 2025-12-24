import { expect, test } from 'vitest';

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
  expect(response.status).toBe(403);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "must be logged in to perform that action",
        },
      ],
    }
  `);
});

test('returns 404 for unknown crates', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
  expect(response.status).toBe(404);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "Not Found",
        },
      ],
    }
  `);
});

test('returns 404 for unknown versions', async function () {
  await db.crate.create({ name: 'foo' });

  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
  expect(response.status).toBe(404);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "Not Found",
        },
      ],
    }
  `);
});

test('yanks the version', async function () {
  let crate = await db.crate.create({ name: 'foo' });
  let version = await db.version.create({ crate, num: '1.0.0', yanked: false });
  expect(version.yanked).toBe(false);
  expect(version.yank_message).toBe(null);

  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "version": {
        "crate": "foo",
        "crate_size": 162963,
        "created_at": "2010-06-16T21:30:45Z",
        "dl_path": "/api/v1/crates/foo/1.0.0/download",
        "downloads": 3702,
        "features": {},
        "id": 1,
        "license": "MIT",
        "linecounts": {
          "languages": {
            "JavaScript": {
              "code_lines": 325,
              "comment_lines": 80,
              "files": 8,
            },
            "TypeScript": {
              "code_lines": 195,
              "comment_lines": 10,
              "files": 2,
            },
          },
          "total_code_lines": 520,
          "total_comment_lines": 90,
        },
        "links": {
          "dependencies": "/api/v1/crates/foo/1.0.0/dependencies",
          "version_downloads": "/api/v1/crates/foo/1.0.0/downloads",
        },
        "num": "1.0.0",
        "published_by": null,
        "readme_path": "/api/v1/crates/foo/1.0.0/readme",
        "rust_version": null,
        "trustpub_data": null,
        "updated_at": "2017-02-24T12:34:56Z",
        "yank_message": "some reason",
        "yanked": true,
      },
    }
  `);

  version = db.version.findFirst(q => q.where({ id: version.id }));
  expect(version.yanked).toBe(true);
  expect(version.yank_message).toBe('some reason');

  response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: UNYANK_BODY });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "version": {
        "crate": "foo",
        "crate_size": 162963,
        "created_at": "2010-06-16T21:30:45Z",
        "dl_path": "/api/v1/crates/foo/1.0.0/download",
        "downloads": 3702,
        "features": {},
        "id": 1,
        "license": "MIT",
        "linecounts": {
          "languages": {
            "JavaScript": {
              "code_lines": 325,
              "comment_lines": 80,
              "files": 8,
            },
            "TypeScript": {
              "code_lines": 195,
              "comment_lines": 10,
              "files": 2,
            },
          },
          "total_code_lines": 520,
          "total_comment_lines": 90,
        },
        "links": {
          "dependencies": "/api/v1/crates/foo/1.0.0/dependencies",
          "version_downloads": "/api/v1/crates/foo/1.0.0/downloads",
        },
        "num": "1.0.0",
        "published_by": null,
        "readme_path": "/api/v1/crates/foo/1.0.0/readme",
        "rust_version": null,
        "trustpub_data": null,
        "updated_at": "2017-02-24T12:34:56Z",
        "yank_message": null,
        "yanked": false,
      },
    }
  `);

  version = db.version.findFirst(q => q.where({ id: version.id }));
  expect(version.yanked).toBe(false);
  expect(version.yank_message).toBe(null);
});
