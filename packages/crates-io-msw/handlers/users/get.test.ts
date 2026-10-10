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
  let body = await response.json();
  expect(body).not.toHaveProperty('linked_accounts');
  expect(body).toMatchInlineSnapshot(`
    {
      "user": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "created_at": null,
        "github_username_matches": true,
        "id": 1,
        "login": "crates-user",
        "name": "User 1",
        "url": "https://github.com/github-user",
      },
    }
  `);

  let expandedResponse = await fetch('/api/v1/users/Crates_User?include=linked_accounts');
  expect(expandedResponse.status).toBe(200);
  expect(await expandedResponse.json()).toMatchInlineSnapshot(`
    {
      "linked_accounts": [
        {
          "account_id": "10",
          "avatar": null,
          "login": "github-user",
          "provider": "github",
        },
        {
          "account_id": "11",
          "avatar": null,
          "login": "crates-user",
          "provider": "github",
        },
      ],
      "user": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "created_at": null,
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
        "created_at": null,
        "github_username_matches": true,
        "id": 2,
        "login": "second-user",
        "name": "User 2",
        "url": "https://github.com/github-user",
      },
    }
  `);
});

test.each([
  ['9', '10', false],
  ['10', '9', true],
  ['9007199254740992', '9007199254740993', false],
  ['9007199254740993', '9007199254740992', true],
])('resolves reused GitHub logins by account ID (%s vs %s)', async function (accountId, otherAccountId, expected) {
  await db.user.create({
    login: 'bob',
    githubAccounts: [{ accountId: otherAccountId, login: 'ALICE', avatar: null }],
  });
  await db.user.create({
    login: 'alice',
    githubAccounts: [{ accountId, login: 'alice', avatar: null }],
  });

  let response = await fetch('/api/v1/users/alice');
  expect(response.status).toBe(200);
  let body = await response.json();
  expect(body.user.github_username_matches).toBe(expected);
});

test.each([{ githubAccounts: [] }, { githubAccounts: [{ accountId: '1', login: 'alice_smith', avatar: null }] }])(
  'reports no GitHub match for missing links or distinct separators (%j)',
  async function ({ githubAccounts }) {
    await db.user.create({ login: 'alice-smith', githubAccounts });

    let response = await fetch('/api/v1/users/alice-smith');
    expect(response.status).toBe(200);
    let body = await response.json();
    expect(body.user.github_username_matches).toBe(false);
  },
);

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
        "created_at": null,
        "github_username_matches": true,
        "id": 2,
        "login": "FOO_BAR",
        "name": "Newer User",
        "url": "https://github.com/FOO_BAR",
      },
    }
  `);
});
