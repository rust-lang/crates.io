import { test } from 'vitest';

import { db } from '../index.js';

test('default are applied', async ({ expect }) => {
  let user = await db.user.create({});
  expect(user).toMatchInlineSnapshot(`
    {
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
    }
  `);
});

test('name can be set', async ({ expect }) => {
  let user = await db.user.create({ name: 'John Doe' });
  expect(user).toMatchInlineSnapshot(`
    {
      "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
      "email": "john-doe@crates.io",
      "emailVerificationToken": null,
      "emailVerified": true,
      "followedCrates": [],
      "id": 1,
      "isAdmin": false,
      "login": "john-doe",
      "name": "John Doe",
      "publishNotifications": true,
      "url": "https://github.com/john-doe",
    }
  `);
});
