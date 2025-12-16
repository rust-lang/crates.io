import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('empty case', async function () {
  let response = await fetch('/api/v1/crates');
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
    crates: [],
    meta: {
      total: 0,
    },
  });
});

test('returns a paginated crates list', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({
    crate,
    created_at: '2020-11-06T12:34:56Z',
    num: '1.0.0',
    updated_at: '2020-11-06T12:34:56Z',
  });
  await db.version.create({
    crate,
    created_at: '2020-12-25T12:34:56Z',
    num: '2.0.0-beta.1',
    updated_at: '2020-12-25T12:34:56Z',
  });

  let response = await fetch('/api/v1/crates');
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
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
        num_versions: 2,
        repository: null,
        recent_downloads: 321,
        trustpub_only: false,
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
  let crates = await Promise.all(Array.from({ length: 25 }, () => db.crate.create()));
  await Promise.all(crates.map(crate => db.version.create({ crate })));

  let response = await fetch('/api/v1/crates');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.crates.length).toBe(10);
  expect(responsePayload.meta.total).toBe(25);
});

test('supports `page` and `per_page` parameters', async function () {
  let crates = await Promise.all(
    Array.from({ length: 25 }, (_, i) => db.crate.create({ name: `crate-${String(i + 1).padStart(2, '0')}` })),
  );
  await Promise.all(crates.map(crate => db.version.create({ crate })));

  let response = await fetch('/api/v1/crates?page=2&per_page=5');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.crates.length).toBe(5);
  expect(responsePayload.crates.map(it => it.id)).toEqual(['crate-06', 'crate-07', 'crate-08', 'crate-09', 'crate-10']);
  expect(responsePayload.meta.total).toBe(25);
});

test('supports a `letter` parameter', async function () {
  let foo = await db.crate.create({ name: 'foo' });
  await db.version.create({ crate: foo });
  let bar = await db.crate.create({ name: 'bar' });
  await db.version.create({ crate: bar });
  let baz = await db.crate.create({ name: 'BAZ' });
  await db.version.create({ crate: baz });

  let response = await fetch('/api/v1/crates?letter=b');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.crates.length).toBe(2);
  expect(responsePayload.crates.map(it => it.id)).toEqual(['bar', 'BAZ']);
  expect(responsePayload.meta.total).toBe(2);
});

test('supports a `q` parameter', async function () {
  let crate1 = await db.crate.create({ name: '123456' });
  await db.version.create({ crate: crate1 });
  let crate2 = await db.crate.create({ name: '123' });
  await db.version.create({ crate: crate2 });
  let crate3 = await db.crate.create({ name: '87654' });
  await db.version.create({ crate: crate3 });

  let response = await fetch('/api/v1/crates?q=123');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.crates.length).toBe(2);
  expect(responsePayload.crates.map(it => it.id)).toEqual(['123456', '123']);
  expect(responsePayload.crates.map(it => it.exact_match)).toEqual([false, true]);
  expect(responsePayload.meta.total).toBe(2);
});

test('supports a `user_id` parameter', async function () {
  let user1 = await db.user.create();
  let user2 = await db.user.create();

  let foo = await db.crate.create({ name: 'foo' });
  await db.version.create({ crate: foo });
  let bar = await db.crate.create({ name: 'bar' });
  await db.crateOwnership.create({ crate: bar, user: user1 });
  await db.version.create({ crate: bar });
  let baz = await db.crate.create({ name: 'baz' });
  await db.crateOwnership.create({ crate: baz, user: user2 });
  await db.version.create({ crate: baz });

  let response = await fetch(`/api/v1/crates?user_id=${user1.id}`);
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.crates.length).toBe(1);
  expect(responsePayload.crates[0].id).toBe('bar');
  expect(responsePayload.meta.total).toBe(1);
});

test('supports a `team_id` parameter', async function () {
  let team1 = await db.team.create();
  let team2 = await db.team.create();

  let foo = await db.crate.create({ name: 'foo' });
  await db.version.create({ crate: foo });
  let bar = await db.crate.create({ name: 'bar' });
  await db.crateOwnership.create({ crate: bar, team: team1 });
  await db.version.create({ crate: bar });
  let baz = await db.crate.create({ name: 'baz' });
  await db.crateOwnership.create({ crate: baz, team: team2 });
  await db.version.create({ crate: baz });

  let response = await fetch(`/api/v1/crates?team_id=${team1.id}`);
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.crates.length).toBe(1);
  expect(responsePayload.crates[0].id).toBe('bar');
  expect(responsePayload.meta.total).toBe(1);
});

test('supports a `following` parameter', async function () {
  let foo = await db.crate.create({ name: 'foo' });
  await db.version.create({ crate: foo });
  let bar = await db.crate.create({ name: 'bar' });
  await db.version.create({ crate: bar });

  let user = await db.user.create({ followedCrates: [bar] });
  await db.mswSession.create({ user });

  let response = await fetch(`/api/v1/crates?following=1`);
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.crates.length).toBe(1);
  expect(responsePayload.crates[0].id).toBe('bar');
  expect(responsePayload.meta.total).toBe(1);
});

test('supports multiple `ids[]` parameters', async function () {
  let foo = await db.crate.create({ name: 'foo' });
  await db.version.create({ crate: foo });
  let bar = await db.crate.create({ name: 'bar' });
  await db.version.create({ crate: bar });
  let baz = await db.crate.create({ name: 'baz' });
  await db.version.create({ crate: baz });
  let other = await db.crate.create({ name: 'other' });
  await db.version.create({ crate: other });

  let response = await fetch(`/api/v1/crates?ids[]=foo&ids[]=bar&ids[]=baz&ids[]=baz&ids[]=unknown`);
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.crates.length).toBe(3);
  expect(responsePayload.crates[0].id).toBe('foo');
  expect(responsePayload.crates[1].id).toBe('bar');
  expect(responsePayload.crates[2].id).toBe('baz');
  expect(responsePayload.meta.total).toBe(3);
});
