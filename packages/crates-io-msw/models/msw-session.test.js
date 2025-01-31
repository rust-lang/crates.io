import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `user` is not set', ({ expect }) => {
  expect(() => db.mswSession.create()).toThrowErrorMatchingInlineSnapshot(`[Error: Missing \`user\` relationship]`);
});

test('happy path', ({ expect }) => {
  let user = db.user.create();
  let session = db.mswSession.create({ user });
  expect(session).toMatchInlineSnapshot(`
    {
      "id": 1,
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
      Symbol(type): "mswSession",
      Symbol(primaryKey): "id",
    }
  `);
});
