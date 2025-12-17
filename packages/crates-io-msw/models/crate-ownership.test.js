import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `crate` is not set', async ({ expect }) => {
  let user = await db.user.create({});
  await expect(() => db.crateOwnership.create({ user })).rejects.toThrowErrorMatchingInlineSnapshot(
    `[Error: Failed to create a new record with initial values: does not match the schema. Please see the schema validation errors above.]`,
  );
});

test('throws if `team` and `user` are not set', async ({ expect }) => {
  let crate = await db.crate.create({});
  await expect(() => db.crateOwnership.create({ crate })).rejects.toThrowErrorMatchingInlineSnapshot(
    `[Error: Failed to create a new record with initial values: does not match the schema. Please see the schema validation errors above.]`,
  );
});

test('throws if `team` and `user` are both set', async ({ expect }) => {
  let crate = await db.crate.create({});
  let team = await db.team.create({});
  let user = await db.user.create({});
  await expect(() => db.crateOwnership.create({ crate, team, user })).rejects.toThrowErrorMatchingInlineSnapshot(
    `[Error: Failed to create a new record with initial values: does not match the schema. Please see the schema validation errors above.]`,
  );
});

test('can set `team`', async ({ expect }) => {
  let crate = await db.crate.create({});
  let team = await db.team.create({});
  let ownership = await db.crateOwnership.create({ crate, team });
  expect(ownership).toMatchInlineSnapshot(`
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
      "emailNotifications": true,
      "id": 1,
      "team": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "id": 1,
        "login": "github:rust-lang:team-1",
        "name": "team-1",
        "org": "rust-lang",
        "url": "https://github.com/rust-lang",
      },
      "user": null,
    }
  `);
});

test('can set `user`', async ({ expect }) => {
  let crate = await db.crate.create({});
  let user = await db.user.create({});
  let ownership = await db.crateOwnership.create({ crate, user });
  expect(ownership).toMatchInlineSnapshot(`
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
      "emailNotifications": true,
      "id": 1,
      "team": null,
      "user": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "email": "user-1@crates.io",
        "emailVerificationToken": null,
        "emailVerified": true,
        "followedCrates": [],
        "id": 1,
        "isAdmin": false,
        "login": "user-1",
        "name": "User 1",
        "publishNotifications": true,
        "url": "https://github.com/user-1",
      },
    }
  `);
});
