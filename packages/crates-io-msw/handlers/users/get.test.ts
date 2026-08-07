import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown users', async function () {
  let response = await fetch('/api/v1/users/foo');
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

test('returns a user object for known users', async function () {
  await db.user.create({
    login: 'crates-user',
    githubAccounts: [
      { accountId: '10', login: 'github-user', avatar: null },
      { accountId: '11', login: 'crates-user', avatar: null },
    ],
  });

  let response = await fetch('/api/v1/users/Crates_User');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "user": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "github_username_matches": true,
        "id": 1,
        "login": "crates-user",
        "name": "User 1",
        "url": "https://github.com/github-user",
      },
    }
  `);

  await db.user.create({
    login: 'second-user',
    githubAccounts: [
      { accountId: '20', login: 'github-user', avatar: null },
      { accountId: '21', login: 'SECOND-USER', avatar: null },
    ],
  });

  response = await fetch('/api/v1/users/second-user');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "user": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "github_username_matches": false,
        "id": 2,
        "login": "second-user",
        "name": "User 2",
        "url": "https://github.com/github-user",
      },
    }
  `);
});

test('returns the newest user for canonical username collisions', async function () {
  await db.user.create({
    login: 'foo-bar',
    name: 'Older User',
  });
  await db.user.create({
    login: 'FOO_BAR',
    name: 'Newer User',
  });

  let response = await fetch('/api/v1/users/Foo-Bar');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "user": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "github_username_matches": true,
        "id": 2,
        "login": "FOO_BAR",
        "name": "Newer User",
        "url": "https://github.com/FOO_BAR",
      },
    }
  `);
});
