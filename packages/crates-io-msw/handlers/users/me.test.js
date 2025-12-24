import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns the `user` resource including the private fields', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/me');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "owned_crates": [],
      "user": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "email": "user-1@crates.io",
        "email_verification_sent": true,
        "email_verified": true,
        "id": 1,
        "is_admin": false,
        "login": "user-1",
        "name": "User 1",
        "publish_notifications": true,
        "url": "https://github.com/user-1",
      },
    }
  `);
});

test('returns a list of `owned_crates`', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let [crate1, , crate3] = await Promise.all(Array.from({ length: 3 }, () => db.crate.create({})));

  await db.crateOwnership.create({ crate: crate1, user });
  await db.crateOwnership.create({ crate: crate3, user });

  let response = await fetch('/api/v1/me');
  expect(response.status).toBe(200);

  let responsePayload = await response.json();
  expect(responsePayload.owned_crates).toMatchInlineSnapshot(`
    [
      {
        "email_notifications": true,
        "id": 1,
        "name": "crate-1",
      },
      {
        "email_notifications": true,
        "id": 3,
        "name": "crate-3",
      },
    ]
  `);
});

test('returns an error if unauthenticated', async function () {
  await db.user.create({});

  let response = await fetch('/api/v1/me');
  expect(response.status).toBe(403);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "must be logged in to perform that action",
        },
      ],
    }
  `);
});
