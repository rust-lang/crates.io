import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/versions');
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

  let response = await fetch('/api/v1/crates/rand/versions');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "meta": {
        "next_page": null,
        "total": 0,
      },
      "versions": [],
    }
  `);
});

test('returns all versions belonging to the specified crate', async function () {
  let user = await db.user.create();
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });
  await db.version.create({ crate, num: '1.1.0', publishedBy: user });
  await db.version.create({ crate, num: '1.2.0', rust_version: '1.69' });

  let response = await fetch('/api/v1/crates/rand/versions');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "meta": {
        "next_page": null,
        "total": 3,
      },
      "versions": [
        {
          "crate": "rand",
          "crate_size": 488889,
          "created_at": "2010-06-16T21:30:45Z",
          "dl_path": "/api/v1/crates/rand/1.2.0/download",
          "downloads": 11106,
          "features": {},
          "id": 3,
          "license": "MIT/Apache-2.0",
          "linecounts": {
            "languages": {
              "Python": {
                "code_lines": 421,
                "comment_lines": 64,
                "files": 8,
              },
            },
            "total_code_lines": 421,
            "total_comment_lines": 64,
          },
          "links": {
            "dependencies": "/api/v1/crates/rand/1.2.0/dependencies",
            "version_downloads": "/api/v1/crates/rand/1.2.0/downloads",
          },
          "num": "1.2.0",
          "published_by": null,
          "readme_path": "/api/v1/crates/rand/1.2.0/readme",
          "rust_version": "1.69",
          "trustpub_data": null,
          "updated_at": "2017-02-24T12:34:56Z",
          "yank_message": null,
          "yanked": false,
        },
        {
          "crate": "rand",
          "crate_size": 325926,
          "created_at": "2010-06-16T21:30:45Z",
          "dl_path": "/api/v1/crates/rand/1.1.0/download",
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
            "dependencies": "/api/v1/crates/rand/1.1.0/dependencies",
            "version_downloads": "/api/v1/crates/rand/1.1.0/downloads",
          },
          "num": "1.1.0",
          "published_by": {
            "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
            "id": 1,
            "login": "user-1",
            "name": "User 1",
            "url": "https://github.com/user-1",
          },
          "readme_path": "/api/v1/crates/rand/1.1.0/readme",
          "rust_version": null,
          "trustpub_data": null,
          "updated_at": "2017-02-24T12:34:56Z",
          "yank_message": null,
          "yanked": false,
        },
        {
          "crate": "rand",
          "crate_size": 162963,
          "created_at": "2010-06-16T21:30:45Z",
          "dl_path": "/api/v1/crates/rand/1.0.0/download",
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
            "dependencies": "/api/v1/crates/rand/1.0.0/dependencies",
            "version_downloads": "/api/v1/crates/rand/1.0.0/downloads",
          },
          "num": "1.0.0",
          "published_by": null,
          "readme_path": "/api/v1/crates/rand/1.0.0/readme",
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

test('supports `sort` parameters', async function () {
  let user = await db.user.create();
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });
  await db.version.create({ crate, num: '2.0.0-alpha', publishedBy: user });
  await db.version.create({ crate, num: '1.1.0', rust_version: '1.69' });

  // sort by `semver` by default
  {
    let response = await fetch('/api/v1/crates/rand/versions');
    expect(response.status).toBe(200);
    let json = await response.json();
    expect(json.versions.map(it => it.num)).toMatchInlineSnapshot(`
      [
        "2.0.0-alpha",
        "1.1.0",
        "1.0.0",
      ]
    `);
  }

  {
    let response = await fetch('/api/v1/crates/rand/versions?sort=semver');
    expect(response.status).toBe(200);
    let json = await response.json();
    expect(json.versions.map(it => it.num)).toMatchInlineSnapshot(`
      [
        "2.0.0-alpha",
        "1.1.0",
        "1.0.0",
      ]
    `);
  }

  {
    let response = await fetch('/api/v1/crates/rand/versions?sort=date');
    expect(response.status).toBe(200);
    let json = await response.json();
    expect(json.versions.map(it => it.num)).toMatchInlineSnapshot(`
      [
        "1.1.0",
        "2.0.0-alpha",
        "1.0.0",
      ]
    `);
  }
});

test('supports multiple `ids[]` parameters', async function () {
  let user = await db.user.create();
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });
  await db.version.create({ crate, num: '1.1.0', publishedBy: user });
  await db.version.create({ crate, num: '1.2.0', rust_version: '1.69' });
  let response = await fetch('/api/v1/crates/rand/versions?nums[]=1.0.0&nums[]=1.2.0');
  expect(response.status).toBe(200);
  let json = await response.json();
  expect(json.versions.map(v => v.num)).toMatchInlineSnapshot(`
    [
      "1.2.0",
      "1.0.0",
    ]
  `);
});

test('supports seek pagination', async function () {
  let user = await db.user.create();
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });
  await db.version.create({ crate, num: '2.0.0-alpha', publishedBy: user });
  await db.version.create({ crate, num: '1.1.0', rust_version: '1.69' });

  async function seek_forwards(queryParams) {
    let calls = 0;
    let next_page;
    let responses = [];
    let base_url = '/api/v1/crates/rand/versions';
    let params = new URLSearchParams(queryParams);
    let url = `${base_url}?${params}`;
    while ((calls == 0 || next_page) && calls < 50) {
      if (next_page) {
        url = `${base_url}${next_page}`;
      }
      let response = await fetch(url);
      calls += 1;
      expect(response.status).toBe(200);
      let json = await response.json();
      responses.push(json);
      next_page = json.meta.next_page;
      if (next_page == null) {
        break;
      }
    }
    return responses;
  }

  // sort by `semver` by default
  {
    let responses = await seek_forwards({ per_page: 1 });
    expect(responses.map(it => it.versions.map(v => v.num))).toMatchInlineSnapshot(`
      [
        [
          "2.0.0-alpha",
        ],
        [
          "1.1.0",
        ],
        [
          "1.0.0",
        ],
        [],
      ]
    `);
    expect(responses.map(it => it.meta.next_page)).toMatchInlineSnapshot(`
      [
        "?per_page=1&seek=2.0.0-alpha",
        "?per_page=1&seek=1.1.0",
        "?per_page=1&seek=1.0.0",
        null,
      ]
    `);
  }

  {
    let responses = await seek_forwards({ per_page: 1, sort: 'semver' });
    expect(responses.map(it => it.versions.map(v => v.num))).toMatchInlineSnapshot(`
      [
        [
          "2.0.0-alpha",
        ],
        [
          "1.1.0",
        ],
        [
          "1.0.0",
        ],
        [],
      ]
    `);
    expect(responses.map(it => it.meta.next_page)).toMatchInlineSnapshot(`
      [
        "?per_page=1&sort=semver&seek=2.0.0-alpha",
        "?per_page=1&sort=semver&seek=1.1.0",
        "?per_page=1&sort=semver&seek=1.0.0",
        null,
      ]
    `);
  }

  {
    let responses = await seek_forwards({ per_page: 1, sort: 'date' });
    expect(responses.map(it => it.versions.map(v => v.num))).toMatchInlineSnapshot(`
      [
        [
          "1.1.0",
        ],
        [
          "2.0.0-alpha",
        ],
        [
          "1.0.0",
        ],
        [],
      ]
    `);
    expect(responses.map(it => it.meta.next_page)).toMatchInlineSnapshot(`
      [
        "?per_page=1&sort=date&seek=1.1.0",
        "?per_page=1&sort=date&seek=2.0.0-alpha",
        "?per_page=1&sort=date&seek=1.0.0",
        null,
      ]
    `);
  }
});

test('include `release_tracks` meta', async function () {
  let user = await db.user.create();
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '0.0.1' });
  await db.version.create({ crate, num: '0.0.2', yanked: true });
  await db.version.create({ crate, num: '1.0.0' });
  await db.version.create({ crate, num: '1.1.0', publishedBy: user });
  await db.version.create({ crate, num: '1.2.0', rust_version: '1.69', yanked: true });

  let req = await fetch('/api/v1/crates/rand/versions');
  let expected = await req.json();

  let response = await fetch('/api/v1/crates/rand/versions?include=release_tracks');
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
    ...expected,
    meta: {
      ...expected.meta,
      release_tracks: {
        '0.0': {
          highest: '0.0.1',
        },
        1: {
          highest: '1.1.0',
        },
      },
    },
  });
});
