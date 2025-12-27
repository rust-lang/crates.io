import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
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

test('empty case', async function () {
  await db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/reverse_dependencies');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "dependencies": [],
      "meta": {
        "total": 0,
      },
      "versions": [],
    }
  `);
});

test('returns a paginated list of crate versions depending to the specified crate', async function () {
  let crate = await db.crate.create({ name: 'foo' });

  await db.dependency.create({
    crate,
    version: await db.version.create({
      crate: await db.crate.create({ name: 'bar' }),
    }),
  });

  await db.dependency.create({
    crate,
    version: await db.version.create({
      crate: await db.crate.create({ name: 'baz' }),
    }),
  });

  let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "dependencies": [
        {
          "crate_id": "foo",
          "default_features": false,
          "features": [],
          "id": 2,
          "kind": "normal",
          "optional": true,
          "req": "0.3.7",
          "target": null,
          "version_id": 2,
        },
        {
          "crate_id": "foo",
          "default_features": false,
          "features": [],
          "id": 1,
          "kind": "normal",
          "optional": true,
          "req": "^2.1.3",
          "target": null,
          "version_id": 1,
        },
      ],
      "meta": {
        "total": 2,
      },
      "versions": [
        {
          "crate": "baz",
          "crate_size": 325926,
          "created_at": "2010-06-16T21:30:45Z",
          "dl_path": "/api/v1/crates/baz/1.0.1/download",
          "downloads": 7404,
          "features": {},
          "id": 2,
          "license": "Apache-2.0",
          "linecounts": {
            "languages": {
              "CSS": {
                "code_lines": 503,
                "comment_lines": 42,
                "files": 2,
              },
              "Python": {
                "code_lines": 284,
                "comment_lines": 91,
                "files": 3,
              },
              "TypeScript": {
                "code_lines": 332,
                "comment_lines": 83,
                "files": 7,
              },
            },
            "total_code_lines": 1119,
            "total_comment_lines": 216,
          },
          "links": {
            "dependencies": "/api/v1/crates/baz/1.0.1/dependencies",
            "version_downloads": "/api/v1/crates/baz/1.0.1/downloads",
          },
          "num": "1.0.1",
          "published_by": null,
          "readme_path": "/api/v1/crates/baz/1.0.1/readme",
          "rust_version": null,
          "trustpub_data": null,
          "updated_at": "2017-02-24T12:34:56Z",
          "yank_message": null,
          "yanked": false,
        },
        {
          "crate": "bar",
          "crate_size": 162963,
          "created_at": "2010-06-16T21:30:45Z",
          "dl_path": "/api/v1/crates/bar/1.0.0/download",
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
            "dependencies": "/api/v1/crates/bar/1.0.0/dependencies",
            "version_downloads": "/api/v1/crates/bar/1.0.0/downloads",
          },
          "num": "1.0.0",
          "published_by": null,
          "readme_path": "/api/v1/crates/bar/1.0.0/readme",
          "rust_version": null,
          "trustpub_data": null,
          "updated_at": "2017-02-24T12:34:56Z",
          "yank_message": null,
          "yanked": false,
        },
      ],
    }
  `);
});

test('never returns more than 10 results', async function () {
  let crate = await db.crate.create({ name: 'foo' });

  await Promise.all(
    Array.from({ length: 25 }, async () => {
      let depCrate = await db.crate.create({ name: 'bar' });
      let version = await db.version.create({ crate: depCrate });
      return db.dependency.create({ crate, version });
    }),
  );

  let response = await fetch('/api/v1/crates/foo/reverse_dependencies');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.dependencies.length).toBe(10);
  expect(responsePayload.versions.length).toBe(10);
  expect(responsePayload.meta.total).toBe(25);
});

test('supports `page` and `per_page` parameters', async function () {
  let crate = await db.crate.create({ name: 'foo' });

  let crates = await Promise.all(
    Array.from({ length: 25 }, (_, i) => db.crate.create({ name: `crate-${String(i + 1).padStart(2, '0')}` })),
  );
  let versions = await Promise.all(crates.map(crate => db.version.create({ crate })));
  await Promise.all(versions.map(version => db.dependency.create({ crate, version })));

  let response = await fetch('/api/v1/crates/foo/reverse_dependencies?page=2&per_page=5');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.dependencies.length).toBe(5);
  expect(responsePayload.versions.map(it => it.crate)).toMatchInlineSnapshot(`
    [
      "crate-24",
      "crate-02",
      "crate-15",
      "crate-06",
      "crate-19",
    ]
  `);
  expect(responsePayload.meta.total).toBe(25);
});
