import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `crate` is not set', ({ expect }) => {
  let inviter = db.user.create();
  let invitee = db.user.create();
  expect(() => db.crateOwnerInvitation.create({ inviter, invitee })).toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`crate\` relationship on \`crate-owner-invitation\`]`,
  );
});

test('throws if `inviter` is not set', ({ expect }) => {
  let crate = db.crate.create();
  let invitee = db.user.create();
  expect(() => db.crateOwnerInvitation.create({ crate, invitee })).toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`inviter\` relationship on \`crate-owner-invitation\`]`,
  );
});

test('throws if `invitee` is not set', ({ expect }) => {
  let crate = db.crate.create();
  let inviter = db.user.create();
  expect(() => db.crateOwnerInvitation.create({ crate, inviter })).toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`invitee\` relationship on \`crate-owner-invitation\`]`,
  );
});

test('happy path', ({ expect }) => {
  let crate = db.crate.create();
  let inviter = db.user.create();
  let invitee = db.user.create();
  let invite = db.crateOwnerInvitation.create({ crate, inviter, invitee });
  expect(invite).toMatchInlineSnapshot(`
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
      "createdAt": "2016-12-24T12:34:56Z",
      "expiresAt": "2017-01-24T12:34:56Z",
      "id": 1,
      "invitee": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "email": "user-2@crates.io",
        "emailVerificationToken": null,
        "emailVerified": true,
        "followedCrates": [],
        "id": 2,
        "isAdmin": false,
        "login": "user-2",
        "name": "User 2",
        "publishNotifications": true,
        "url": "https://github.com/user-2",
        Symbol(type): "user",
        Symbol(primaryKey): "id",
      },
      "inviter": {
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
      "token": "secret-token-1",
      Symbol(type): "crateOwnerInvitation",
      Symbol(primaryKey): "id",
    }
  `);
});
