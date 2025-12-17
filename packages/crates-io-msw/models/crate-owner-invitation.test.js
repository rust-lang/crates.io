import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `crate` is not set', async ({ expect }) => {
  let inviter = await db.user.create({});
  let invitee = await db.user.create({});
  await expect(() => db.crateOwnerInvitation.create({ inviter, invitee })).rejects.toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`crate\` relationship on \`crate-owner-invitation\`]`,
  );
});

test('throws if `inviter` is not set', async ({ expect }) => {
  let crate = await db.crate.create({});
  let invitee = await db.user.create({});
  await expect(() => db.crateOwnerInvitation.create({ crate, invitee })).rejects.toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`inviter\` relationship on \`crate-owner-invitation\`]`,
  );
});

test('throws if `invitee` is not set', async ({ expect }) => {
  let crate = await db.crate.create({});
  let inviter = await db.user.create({});
  await expect(() => db.crateOwnerInvitation.create({ crate, inviter })).rejects.toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`invitee\` relationship on \`crate-owner-invitation\`]`,
  );
});

test('happy path', async ({ expect }) => {
  let crate = await db.crate.create({});
  let inviter = await db.user.create({});
  let invitee = await db.user.create({});
  let invite = await db.crateOwnerInvitation.create({ crate, inviter, invitee });
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
        "trustpubOnly": false,
        "updated_at": "2017-02-24T12:34:56Z",
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
      },
      "token": "secret-token-1",
    }
  `);
});
