import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `crate` is not set', ({ expect }) => {
  let user = db.user.create();
  expect(() => db.crateOwnership.create({ user })).toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`crate\` relationship on \`crate-ownership\`]`,
  );
});

test('throws if `team` and `user` are not set', ({ expect }) => {
  let crate = db.crate.create();
  expect(() => db.crateOwnership.create({ crate })).toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`team\` or \`user\` relationship on \`crate-ownership\`]`,
  );
});

test('throws if `team` and `user` are both set', ({ expect }) => {
  let crate = db.crate.create();
  let team = db.team.create();
  let user = db.user.create();
  expect(() => db.crateOwnership.create({ crate, team, user })).toThrowErrorMatchingInlineSnapshot(
    `[Error: \`team\` and \`user\` on a \`crate-ownership\` are mutually exclusive]`,
  );
});

test('can set `team`', ({ expect }) => {
  let crate = db.crate.create();
  let team = db.team.create();
  let ownership = db.crateOwnership.create({ crate, team });
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
        "updated_at": "2017-02-24T12:34:56Z",
        Symbol(type): "crate",
        Symbol(primaryKey): "id",
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
        Symbol(type): "team",
        Symbol(primaryKey): "id",
      },
      "user": null,
      Symbol(type): "crateOwnership",
      Symbol(primaryKey): "id",
    }
  `);
});

test('can set `user`', ({ expect }) => {
  let crate = db.crate.create();
  let user = db.user.create();
  let ownership = db.crateOwnership.create({ crate, user });
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
        "updated_at": "2017-02-24T12:34:56Z",
        Symbol(type): "crate",
        Symbol(primaryKey): "id",
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
        Symbol(type): "user",
        Symbol(primaryKey): "id",
      },
      Symbol(type): "crateOwnership",
      Symbol(primaryKey): "id",
    }
  `);
});
