import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('empty case', async function () {
  let response = await fetch('/api/v1/crates');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    crates: [],
    meta: {
      total: 0,
    },
  });
});

test('returns a paginated crates list', async function () {
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({
    crate,
    created_at: '2020-11-06T12:34:56Z',
    num: '1.0.0',
    updated_at: '2020-11-06T12:34:56Z',
  });
  db.version.create({
    crate,
    created_at: '2020-12-25T12:34:56Z',
    num: '2.0.0-beta.1',
    updated_at: '2020-12-25T12:34:56Z',
  });

  let response = await fetch('/api/v1/crates');
  // assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    crates: [
      {
        id: 'rand',
        badges: [],
        categories: null,
        created_at: '2010-06-16T21:30:45Z',
        default_version: '1.0.0',
        description: 'This is the description for the crate called "rand"',
        documentation: null,
        downloads: 37_035,
        exact_match: false,
        homepage: null,
        keywords: null,
        links: {
          owner_team: '/api/v1/crates/rand/owner_team',
          owner_user: '/api/v1/crates/rand/owner_user',
          reverse_dependencies: '/api/v1/crates/rand/reverse_dependencies',
          version_downloads: '/api/v1/crates/rand/downloads',
          versions: '/api/v1/crates/rand/versions',
        },
        max_version: '2.0.0-beta.1',
        max_stable_version: '1.0.0',
        name: 'rand',
        newest_version: '2.0.0-beta.1',
        repository: null,
        recent_downloads: 321,
        updated_at: '2017-02-24T12:34:56Z',
        versions: null,
        yanked: false,
      },
    ],
    meta: {
      total: 1,
    },
  });
});

test('never returns more than 10 results', async function () {
  let crates = Array.from({ length: 25 }, () => db.crate.create());
  crates.forEach(crate => db.version.create({ crate }));

  let response = await fetch('/api/v1/crates');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.crates.length, 10);
  assert.strictEqual(responsePayload.meta.total, 25);
});

test('supports `page` and `per_page` parameters', async function () {
  let crates = Array.from({ length: 25 }, (_, i) =>
    db.crate.create({ name: `crate-${String(i + 1).padStart(2, '0')}` }),
  );
  crates.forEach(crate => db.version.create({ crate }));

  let response = await fetch('/api/v1/crates?page=2&per_page=5');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.crates.length, 5);
  assert.deepEqual(
    responsePayload.crates.map(it => it.id),
    ['crate-06', 'crate-07', 'crate-08', 'crate-09', 'crate-10'],
  );
  assert.strictEqual(responsePayload.meta.total, 25);
});

test('supports a `letter` parameter', async function () {
  let foo = db.crate.create({ name: 'foo' });
  db.version.create({ crate: foo });
  let bar = db.crate.create({ name: 'bar' });
  db.version.create({ crate: bar });
  let baz = db.crate.create({ name: 'BAZ' });
  db.version.create({ crate: baz });

  let response = await fetch('/api/v1/crates?letter=b');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.crates.length, 2);
  assert.deepEqual(
    responsePayload.crates.map(it => it.id),
    ['bar', 'BAZ'],
  );
  assert.strictEqual(responsePayload.meta.total, 2);
});

test('supports a `q` parameter', async function () {
  let crate1 = db.crate.create({ name: '123456' });
  db.version.create({ crate: crate1 });
  let crate2 = db.crate.create({ name: '123' });
  db.version.create({ crate: crate2 });
  let crate3 = db.crate.create({ name: '87654' });
  db.version.create({ crate: crate3 });

  let response = await fetch('/api/v1/crates?q=123');
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.crates.length, 2);
  assert.deepEqual(
    responsePayload.crates.map(it => it.id),
    ['123456', '123'],
  );
  assert.deepEqual(
    responsePayload.crates.map(it => it.exact_match),
    [false, true],
  );
  assert.strictEqual(responsePayload.meta.total, 2);
});

test('supports a `user_id` parameter', async function () {
  let user1 = db.user.create();
  let user2 = db.user.create();

  let foo = db.crate.create({ name: 'foo' });
  db.version.create({ crate: foo });
  let bar = db.crate.create({ name: 'bar' });
  db.crateOwnership.create({ crate: bar, user: user1 });
  db.version.create({ crate: bar });
  let baz = db.crate.create({ name: 'baz' });
  db.crateOwnership.create({ crate: baz, user: user2 });
  db.version.create({ crate: baz });

  let response = await fetch(`/api/v1/crates?user_id=${user1.id}`);
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.crates.length, 1);
  assert.strictEqual(responsePayload.crates[0].id, 'bar');
  assert.strictEqual(responsePayload.meta.total, 1);
});

test('supports a `team_id` parameter', async function () {
  let team1 = db.team.create();
  let team2 = db.team.create();

  let foo = db.crate.create({ name: 'foo' });
  db.version.create({ crate: foo });
  let bar = db.crate.create({ name: 'bar' });
  db.crateOwnership.create({ crate: bar, team: team1 });
  db.version.create({ crate: bar });
  let baz = db.crate.create({ name: 'baz' });
  db.crateOwnership.create({ crate: baz, team: team2 });
  db.version.create({ crate: baz });

  let response = await fetch(`/api/v1/crates?team_id=${team1.id}`);
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.crates.length, 1);
  assert.strictEqual(responsePayload.crates[0].id, 'bar');
  assert.strictEqual(responsePayload.meta.total, 1);
});

test('supports a `following` parameter', async function () {
  let foo = db.crate.create({ name: 'foo' });
  db.version.create({ crate: foo });
  let bar = db.crate.create({ name: 'bar' });
  db.version.create({ crate: bar });

  let user = db.user.create({ followedCrates: [bar] });
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/crates?following=1`);
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.crates.length, 1);
  assert.strictEqual(responsePayload.crates[0].id, 'bar');
  assert.strictEqual(responsePayload.meta.total, 1);
});

test('supports multiple `ids[]` parameters', async function () {
  let foo = db.crate.create({ name: 'foo' });
  db.version.create({ crate: foo });
  let bar = db.crate.create({ name: 'bar' });
  db.version.create({ crate: bar });
  let baz = db.crate.create({ name: 'baz' });
  db.version.create({ crate: baz });
  let other = db.crate.create({ name: 'other' });
  db.version.create({ crate: other });

  let response = await fetch(`/api/v1/crates?ids[]=foo&ids[]=bar&ids[]=baz&ids[]=baz&ids[]=unknown`);
  assert.strictEqual(response.status, 200);

  let responsePayload = await response.json();
  assert.strictEqual(responsePayload.crates.length, 3);
  assert.strictEqual(responsePayload.crates[0].id, 'foo');
  assert.strictEqual(responsePayload.crates[1].id, 'bar');
  assert.strictEqual(responsePayload.crates[2].id, 'baz');
  assert.strictEqual(responsePayload.meta.total, 3);
});
