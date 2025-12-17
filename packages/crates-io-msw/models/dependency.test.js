import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `crate` is not set', async ({ expect }) => {
  let version = await db.version.create({ crate: await db.crate.create({}) });
  await expect(() => db.dependency.create({ version })).rejects.toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`crate\` relationship on \`dependency:1\`]`,
  );
});

test('throws if `version` is not set', async ({ expect }) => {
  let crate = await db.crate.create({});
  await expect(() => db.dependency.create({ crate })).rejects.toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`version\` relationship on \`dependency:1\`]`,
  );
});

test('happy path', async ({ expect }) => {
  let crate = await db.crate.create({});
  let version = await db.version.create({ crate: await db.crate.create({}) });
  let dependency = await db.dependency.create({ crate, version });
  expect(dependency).toMatchInlineSnapshot(`
    {
      "crate": {
        "_extra_downloads": [],
        "badges": [],
        "categories": [],
        "created_at": "2010-06-16T21:30:45Z",
        "description": "This is the description for the crate called "crate-1"",
        "documentation": null,
        "downloads": 37035,
        "homepage": null,
        "id": 1,
        "keywords": [],
        "name": "crate-1",
        "recent_downloads": 321,
        "repository": null,
        "trustpubOnly": false,
        "updated_at": "2017-02-24T12:34:56Z",
      },
      "default_features": false,
      "features": [],
      "id": 1,
      "kind": "normal",
      "optional": true,
      "req": "^2.1.3",
      "target": null,
      "version": {
        "crate": {
          "_extra_downloads": [],
          "badges": [],
          "categories": [],
          "created_at": "2010-06-16T21:30:45Z",
          "description": "This is the description for the crate called "crate-2"",
          "documentation": null,
          "downloads": 74070,
          "homepage": null,
          "id": 2,
          "keywords": [],
          "name": "crate-2",
          "recent_downloads": 1926,
          "repository": null,
          "trustpubOnly": false,
          "updated_at": "2017-02-24T12:34:56Z",
        },
        "crate_size": 162963,
        "created_at": "2010-06-16T21:30:45Z",
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
        "num": "1.0.0",
        "publishedBy": null,
        "readme": null,
        "rust_version": null,
        "trustpub_data": null,
        "updated_at": "2017-02-24T12:34:56Z",
        "yank_message": null,
        "yanked": false,
      },
    }
  `);
});
