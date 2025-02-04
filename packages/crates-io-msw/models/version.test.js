import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `crate` is not set', ({ expect }) => {
  expect(() => db.version.create()).toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`crate\` relationship on \`version:1.0.0\`]`,
  );
});

test('happy path', ({ expect }) => {
  let crate = db.crate.create();
  let version = db.version.create({ crate });
  expect(version).toMatchInlineSnapshot(`
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
        "updated_at": "2017-02-24T12:34:56Z",
        Symbol(type): "crate",
        Symbol(primaryKey): "id",
      },
      "crate_size": 162963,
      "created_at": "2010-06-16T21:30:45Z",
      "downloads": 3702,
      "features": {},
      "id": 1,
      "license": "MIT",
      "num": "1.0.0",
      "publishedBy": null,
      "readme": null,
      "rust_version": null,
      "updated_at": "2017-02-24T12:34:56Z",
      "yank_message": null,
      "yanked": false,
      Symbol(type): "version",
      Symbol(primaryKey): "id",
    }
  `);
});
