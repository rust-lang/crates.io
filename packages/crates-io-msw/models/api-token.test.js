import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `user` is not set', ({ expect }) => {
  expect(() => db.apiToken.create()).toThrowErrorMatchingInlineSnapshot(
    `[Error: Missing \`user\` relationship on \`api-token\`]`,
  );
});

test('happy path', ({ expect }) => {
  let user = db.user.create();
  let session = db.apiToken.create({ user });
  expect(session).toMatchInlineSnapshot(`
    {
      "crateScopes": null,
      "createdAt": "2017-11-19T17:59:22Z",
      "endpointScopes": null,
      "expiredAt": null,
      "id": 1,
      "lastUsedAt": null,
      "name": "API Token 1",
      "revoked": false,
      "token": "6270739405881613",
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
      Symbol(type): "apiToken",
      Symbol(primaryKey): "id",
    }
  `);
});
