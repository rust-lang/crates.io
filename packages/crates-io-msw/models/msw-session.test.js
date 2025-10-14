import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `user` is not set', async ({ expect }) => {
  await expect(() => db.mswSession.create()).rejects.toThrowErrorMatchingInlineSnapshot(`[Error: Missing \`user\` relationship]`);
});

test('happy path', async ({ expect }) => {
  let user = await db.user.create();
  let session = await db.mswSession.create({ user });
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
      },
    }
  `);
});
