import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/versions');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('empty case', async function () {
  db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/versions');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    versions: [],
    meta: { total: 0, next_page: null },
  });
});

test('returns all versions belonging to the specified crate', async function () {
  let user = db.user.create();
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '1.0.0' });
  db.version.create({ crate, num: '1.1.0', publishedBy: user });
  db.version.create({ crate, num: '1.2.0', rust_version: '1.69' });

  let response = await fetch('/api/v1/crates/rand/versions');
  // assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    versions: [
      {
        id: 3,
        crate: 'rand',
        crate_size: 488_889,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/rand/1.2.0/download',
        downloads: 11_106,
        features: {},
        license: 'MIT/Apache-2.0',
        links: {
          dependencies: '/api/v1/crates/rand/1.2.0/dependencies',
          version_downloads: '/api/v1/crates/rand/1.2.0/downloads',
        },
        num: '1.2.0',
        published_by: null,
        readme_path: '/api/v1/crates/rand/1.2.0/readme',
        rust_version: '1.69',
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
        yank_message: null,
      },
      {
        id: 2,
        crate: 'rand',
        crate_size: 325_926,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/rand/1.1.0/download',
        downloads: 7404,
        features: {},
        license: 'Apache-2.0',
        links: {
          dependencies: '/api/v1/crates/rand/1.1.0/dependencies',
          version_downloads: '/api/v1/crates/rand/1.1.0/downloads',
        },
        num: '1.1.0',
        published_by: {
          id: 1,
          avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
          login: 'user-1',
          name: 'User 1',
          url: 'https://github.com/user-1',
        },
        readme_path: '/api/v1/crates/rand/1.1.0/readme',
        rust_version: null,
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
        yank_message: null,
      },
      {
        id: 1,
        crate: 'rand',
        crate_size: 162_963,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/rand/1.0.0/download',
        downloads: 3702,
        features: {},
        license: 'MIT',
        links: {
          dependencies: '/api/v1/crates/rand/1.0.0/dependencies',
          version_downloads: '/api/v1/crates/rand/1.0.0/downloads',
        },
        num: '1.0.0',
        published_by: null,
        readme_path: '/api/v1/crates/rand/1.0.0/readme',
        rust_version: null,
        updated_at: '2017-02-24T12:34:56Z',
        yanked: false,
        yank_message: null,
      },
    ],
    meta: { total: 3, next_page: null },
  });
});

test('supports `sort` parameters', async function () {
  let user = db.user.create();
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '1.0.0' });
  db.version.create({ crate, num: '2.0.0-alpha', publishedBy: user });
  db.version.create({ crate, num: '1.1.0', rust_version: '1.69' });

  // sort by `semver` by default
  {
    let response = await fetch('/api/v1/crates/rand/versions');
    assert.strictEqual(response.status, 200);
    let json = await response.json();
    assert.deepEqual(
      json.versions.map(it => it.num),
      ['2.0.0-alpha', '1.1.0', '1.0.0'],
    );
  }

  {
    let response = await fetch('/api/v1/crates/rand/versions?sort=semver');
    assert.strictEqual(response.status, 200);
    let json = await response.json();
    assert.deepEqual(
      json.versions.map(it => it.num),
      ['2.0.0-alpha', '1.1.0', '1.0.0'],
    );
  }

  {
    let response = await fetch('/api/v1/crates/rand/versions?sort=date');
    assert.strictEqual(response.status, 200);
    let json = await response.json();
    assert.deepEqual(
      json.versions.map(it => it.num),
      ['1.1.0', '2.0.0-alpha', '1.0.0'],
    );
  }
});

test('supports multiple `ids[]` parameters', async function () {
  let user = db.user.create();
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '1.0.0' });
  db.version.create({ crate, num: '1.1.0', publishedBy: user });
  db.version.create({ crate, num: '1.2.0', rust_version: '1.69' });
  let response = await fetch('/api/v1/crates/rand/versions?nums[]=1.0.0&nums[]=1.2.0');
  assert.strictEqual(response.status, 200);
  let json = await response.json();
  assert.deepEqual(
    json.versions.map(v => v.num),
    ['1.2.0', '1.0.0'],
  );
});

test('supports seek pagination', async function () {
  let user = db.user.create();
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '1.0.0' });
  db.version.create({ crate, num: '2.0.0-alpha', publishedBy: user });
  db.version.create({ crate, num: '1.1.0', rust_version: '1.69' });

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
      assert.strictEqual(response.status, 200);
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
    assert.deepEqual(
      responses.map(it => it.versions.map(v => v.num)),
      [['2.0.0-alpha'], ['1.1.0'], ['1.0.0'], []],
    );
    assert.deepEqual(
      responses.map(it => it.meta.next_page),
      ['?per_page=1&seek=2.0.0-alpha', '?per_page=1&seek=1.1.0', '?per_page=1&seek=1.0.0', null],
    );
  }

  {
    let responses = await seek_forwards({ per_page: 1, sort: 'semver' });
    assert.deepEqual(
      responses.map(it => it.versions.map(v => v.num)),
      [['2.0.0-alpha'], ['1.1.0'], ['1.0.0'], []],
    );
    assert.deepEqual(
      responses.map(it => it.meta.next_page),
      [
        '?per_page=1&sort=semver&seek=2.0.0-alpha',
        '?per_page=1&sort=semver&seek=1.1.0',
        '?per_page=1&sort=semver&seek=1.0.0',
        null,
      ],
    );
  }

  {
    let responses = await seek_forwards({ per_page: 1, sort: 'date' });
    assert.deepEqual(
      responses.map(it => it.versions.map(v => v.num)),
      [['1.1.0'], ['2.0.0-alpha'], ['1.0.0'], []],
    );
    assert.deepEqual(
      responses.map(it => it.meta.next_page),
      [
        '?per_page=1&sort=date&seek=1.1.0',
        '?per_page=1&sort=date&seek=2.0.0-alpha',
        '?per_page=1&sort=date&seek=1.0.0',
        null,
      ],
    );
  }
});

test('include `release_tracks` meta', async function () {
  let user = db.user.create();
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '0.0.1' });
  db.version.create({ crate, num: '0.0.2', yanked: true });
  db.version.create({ crate, num: '1.0.0' });
  db.version.create({ crate, num: '1.1.0', publishedBy: user });
  db.version.create({ crate, num: '1.2.0', rust_version: '1.69', yanked: true });

  let req = await fetch('/api/v1/crates/rand/versions');
  let expected = await req.json();

  let response = await fetch('/api/v1/crates/rand/versions?include=release_tracks');
  // assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
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
