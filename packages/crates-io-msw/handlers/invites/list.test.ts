import { expect, test } from 'vitest';

import { db } from '../../index.js';

const BASE_URL = '/api/v1/crate_owner_invitations';

test('happy path (invitee_id)', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let inviter = await db.user.create({ name: 'janed' });
  let other = await db.user.create({});

  let nanomsg = await db.crate.create({ name: 'nanomsg' });
  await db.crateOwnerInvitation.create({ crate: nanomsg, invitee: user, inviter });
  await db.crateOwnerInvitation.create({ crate: nanomsg, invitee: other, inviter });

  let expiresAt = '2016-01-24T12:34:56Z';
  let ember = await db.crate.create({ name: 'ember-rs' });
  await db.crateOwnerInvitation.create({ crate: ember, invitee: user, inviter, expiresAt });

  let response = await fetch(`${BASE_URL}?invitee_id=${user.id}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "crate_owner_invitations": [
        {
          "crate_id": 1,
          "crate_name": "nanomsg",
          "created_at": "2016-12-24T12:34:56Z",
          "expires_at": "2099-01-24T12:34:56Z",
          "id": 1,
          "invitee_id": 1,
          "inviter_id": 2,
          "state": "pending",
        },
      ],
      "meta": {
        "next_page": null,
      },
      "users": [
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "created_at": null,
          "github_username_matches": true,
          "id": 1,
          "login": "user-1",
          "name": "User 1",
          "url": "https://github.com/user-1",
        },
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "created_at": null,
          "github_username_matches": true,
          "id": 2,
          "login": "janed",
          "name": "janed",
          "url": "https://github.com/janed",
        },
      ],
    }
  `);
});

test('happy path (crate_name)', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let nanomsg = await db.crate.create({ name: 'nanomsg' });
  await db.crateOwnership.create({ crate: nanomsg, user });
  let ember = await db.crate.create({ name: 'ember-rs' });
  await db.crateOwnership.create({ crate: ember, user });

  let invitee = await db.user.create({ name: 'wycats' });
  await db.crateOwnerInvitation.create({ crate: nanomsg, invitee, inviter: user });
  await db.crateOwnerInvitation.create({ crate: ember, invitee, inviter: user });

  let response = await fetch(`${BASE_URL}?crate_name=nanomsg`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "crate_owner_invitations": [
        {
          "crate_id": 1,
          "crate_name": "nanomsg",
          "created_at": "2016-12-24T12:34:56Z",
          "expires_at": "2099-01-24T12:34:56Z",
          "id": 1,
          "invitee_id": 2,
          "inviter_id": 1,
          "state": "pending",
        },
      ],
      "meta": {
        "next_page": null,
      },
      "users": [
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "created_at": null,
          "github_username_matches": true,
          "id": 1,
          "login": "user-1",
          "name": "User 1",
          "url": "https://github.com/user-1",
        },
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "created_at": null,
          "github_username_matches": true,
          "id": 2,
          "login": "wycats",
          "name": "wycats",
          "url": "https://github.com/wycats",
        },
      ],
    }
  `);
});

test('happy path with pagination (invitee_id)', async function () {
  let inviter = await db.user.create({});
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  for (let i = 0; i < 15; i++) {
    let crate = await db.crate.create({});
    await db.crateOwnerInvitation.create({ crate, invitee: user, inviter });
  }

  let response = await fetch(`${BASE_URL}?invitee_id=${user.id}`);
  expect(response.status).toBe(200);
  let json = await response.json();
  expect(json.crate_owner_invitations.map((invite: { crate_id: number }) => invite.crate_id)).toEqual([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
  ]);
  expect(json.meta.next_page).toMatchInlineSnapshot(`"?invitee_id=2&seek=WzEwLDJd"`);

  response = await fetch(`${BASE_URL}${json.meta.next_page}`);
  expect(response.status).toBe(200);
  json = await response.json();
  expect(json.crate_owner_invitations.map((invite: { crate_id: number }) => invite.crate_id)).toEqual([
    11, 12, 13, 14, 15,
  ]);
  expect(json.meta.next_page).toBe(null);
});

test('returns 400 if query params are missing', async function () {
  let response = await fetch(BASE_URL);
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "missing or invalid filter",
        },
      ],
    }
  `);
});

test('returns 403 if unauthenticated', async function () {
  let response = await fetch(`${BASE_URL}?invitee_id=42`);
  expect(response.status).toBe(403);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "this action requires authentication",
        },
      ],
    }
  `);
});

test('returns 403 if requesting for other user', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch(`${BASE_URL}?invitee_id=${user.id + 1}`);
  expect(response.status).toBe(403);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "only the invitee can query their pending invitations",
        },
      ],
    }
  `);
});

test("returns 404 if crate can't be found", async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch(`${BASE_URL}?crate_name=foo`);
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

test('returns 403 if not a crate owner', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });
  await db.crate.create({ name: 'foo' });

  let response = await fetch(`${BASE_URL}?crate_name=foo`);
  expect(response.status).toBe(403);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "only crate owners can query pending invitations for their crate",
        },
      ],
    }
  `);
});
