import { expect, test } from 'vitest';

import { db } from '../../index.js';

const ACCEPTED = JSON.stringify({ crate_owner_invitation: { state: 'accepted' } });
const DECLINED = JSON.stringify({ crate_owner_invitation: { state: 'declined' } });
const REVOKED = JSON.stringify({ crate_owner_invitation: { state: 'revoked' } });

async function invite({ expiresAt }: { expiresAt?: string } = {}) {
  let crate = await db.crate.create({ name: 'serde' });
  let inviter = await db.user.create({});
  await db.crateOwnership.create({ crate, user: inviter });
  let invitee = await db.user.create({});

  let invite = await db.crateOwnerInvitation.create({ crate, invitee, inviter, expiresAt });
  let url = `/api/v1/crate_owner_invitations/${invite.id}`;
  let owners = () => db.crateOwnership.findMany(q => q.where({ crate: { id: crate.id }, user: { id: invitee.id } }));
  return { inviter, invitee, url, owners };
}

test('can accept an invitation', async function () {
  let { invitee, url, owners } = await invite();
  await db.mswSession.create({ user: invitee });

  let response = await fetch(url, { method: 'PATCH', body: ACCEPTED });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "crate_owner_invitation": {
        "crate_id": 1,
        "crate_name": "serde",
        "created_at": "2016-12-24T12:34:56Z",
        "expires_at": "2099-01-24T12:34:56Z",
        "id": 1,
        "invitee_id": 2,
        "inviter_id": 1,
        "state": "accepted",
      },
    }
  `);
  expect(db.crateOwnerInvitation.findMany()).toHaveLength(0);
  expect(owners()).toHaveLength(1);
});

test('can decline an invitation', async function () {
  let { invitee, url, owners } = await invite();
  await db.mswSession.create({ user: invitee });

  let response = await fetch(url, { method: 'PATCH', body: DECLINED });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "crate_owner_invitation": {
        "crate_id": 1,
        "crate_name": "serde",
        "created_at": "2016-12-24T12:34:56Z",
        "expires_at": "2099-01-24T12:34:56Z",
        "id": 1,
        "invitee_id": 2,
        "inviter_id": 1,
        "state": "declined",
      },
    }
  `);
  expect(db.crateOwnerInvitation.findMany()).toHaveLength(0);
  expect(owners()).toHaveLength(0);
});

test('returns 400 for other states', async function () {
  let { invitee, url } = await invite();
  await db.mswSession.create({ user: invitee });

  let body = JSON.stringify({ crate_owner_invitation: { state: 'pending' } });
  let response = await fetch(url, { method: 'PATCH', body });
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "\`state\` must be \`accepted\` or \`declined\`",
        },
      ],
    }
  `);
  expect(db.crateOwnerInvitation.findMany()).toHaveLength(1);
});

test('returns 410 if the invitation is expired', async function () {
  let { invitee, url, owners } = await invite({ expiresAt: '2016-01-24T12:34:56Z' });
  await db.mswSession.create({ user: invitee });

  let response = await fetch(url, { method: 'PATCH', body: ACCEPTED });
  expect(response.status).toBe(410);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "The invitation to become an owner of the serde crate expired. Please reach out to an owner of the crate to request a new invitation.",
        },
      ],
    }
  `);
  expect(owners()).toHaveLength(0);
});

test('can decline an expired invitation', async function () {
  let { invitee, url } = await invite({ expiresAt: '2016-01-24T12:34:56Z' });
  await db.mswSession.create({ user: invitee });

  let response = await fetch(url, { method: 'PATCH', body: DECLINED });
  expect(response.status).toBe(200);
  expect(db.crateOwnerInvitation.findMany()).toHaveLength(0);
});

test('returns 404 if the invitation does not exist', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crate_owner_invitations/42', { method: 'PATCH', body: ACCEPTED });
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

test('returns 403 if the invitation is for another user', async function () {
  let { url } = await invite();
  let other = await db.user.create({});
  await db.mswSession.create({ user: other });

  let response = await fetch(url, { method: 'PATCH', body: REVOKED });
  expect(response.status).toBe(403);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "only the invitee or a crate owner can update this invitation",
        },
      ],
    }
  `);
  expect(db.crateOwnerInvitation.findMany()).toHaveLength(1);
});

test('can revoke an invitation', async function () {
  let { inviter, url, owners } = await invite();
  await db.mswSession.create({ user: inviter });

  let response = await fetch(url, { method: 'PATCH', body: REVOKED });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "crate_owner_invitation": {
        "crate_id": 1,
        "crate_name": "serde",
        "created_at": "2016-12-24T12:34:56Z",
        "expires_at": "2099-01-24T12:34:56Z",
        "id": 1,
        "invitee_id": 2,
        "inviter_id": 1,
        "state": "revoked",
      },
    }
  `);
  expect(db.crateOwnerInvitation.findMany()).toHaveLength(0);
  expect(owners()).toHaveLength(0);
});

test('returns 400 if a crate owner sends another state', async function () {
  let { inviter, url } = await invite();
  await db.mswSession.create({ user: inviter });

  let response = await fetch(url, { method: 'PATCH', body: ACCEPTED });
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "\`state\` must be \`revoked\`",
        },
      ],
    }
  `);
  expect(db.crateOwnerInvitation.findMany()).toHaveLength(1);
});

test('returns 403 if unauthenticated', async function () {
  let { url } = await invite();

  let response = await fetch(url, { method: 'PATCH', body: ACCEPTED });
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
