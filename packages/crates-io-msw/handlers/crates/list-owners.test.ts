import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/owners');
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

  let response = await fetch('/api/v1/crates/rand/owners');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "users": [],
    }
  `);
});

test('returns user owners', async function () {
  let user = await db.user.create({ name: 'John Doe' });
  let crate = await db.crate.create({ name: 'rand' });
  await db.crateOwnership.create({ crate, user });

  let response = await fetch('/api/v1/crates/rand/owners');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "users": [
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "id": 1,
          "kind": "user",
          "login": "john-doe",
          "name": "John Doe",
          "url": "https://github.com/john-doe",
        },
      ],
    }
  `);
});

test('returns team owners', async function () {
  let team = await db.team.create({ name: 'maintainers' });
  let crate = await db.crate.create({ name: 'rand' });
  await db.crateOwnership.create({ crate, team });

  let response = await fetch('/api/v1/crates/rand/owners');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "users": [
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "id": 1,
          "kind": "team",
          "login": "github:rust-lang:maintainers",
          "name": "maintainers",
          "url": "https://github.com/rust-lang",
        },
      ],
    }
  `);
});

test('returns user owners before team owners', async function () {
  let user = await db.user.create({ name: 'John Doe' });
  let team = await db.team.create({ name: 'maintainers' });
  let crate = await db.crate.create({ name: 'rand' });
  await db.crateOwnership.create({ crate, user });
  await db.crateOwnership.create({ crate, team });

  let response = await fetch('/api/v1/crates/rand/owners');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "users": [
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "id": 1,
          "kind": "user",
          "login": "john-doe",
          "name": "John Doe",
          "url": "https://github.com/john-doe",
        },
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "id": 1,
          "kind": "team",
          "login": "github:rust-lang:maintainers",
          "name": "maintainers",
          "url": "https://github.com/rust-lang",
        },
      ],
    }
  `);
});
