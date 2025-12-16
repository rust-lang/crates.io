import { expect, test } from 'vitest';

import { db } from '../../index.js';

const REMOVE_USER_BODY = JSON.stringify({ owners: ['john-doe'] });

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body: REMOVE_USER_BODY });
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

test('returns 404 for unknown crates', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body: REMOVE_USER_BODY });
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

test('can remove a user owner', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let crate = await db.crate.create({ name: 'foo' });
  await db.crateOwnership.create({ crate, user });

  let user2 = await db.user.create();
  await db.crateOwnership.create({ crate, user: user2 });

  let body = JSON.stringify({ owners: [user2.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "msg": "owners successfully removed",
      "ok": true,
    }
  `);

  let owners = db.crateOwnership.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(owners.length).toBe(1);
  expect(owners[0].user.id).toBe(user.id);

  let invites = db.crateOwnerInvitation.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(invites.length).toBe(0);
});

test('can remove a team owner', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let crate = await db.crate.create({ name: 'foo' });
  await db.crateOwnership.create({ crate, user });

  let team = await db.team.create();
  await db.crateOwnership.create({ crate, team });

  let body = JSON.stringify({ owners: [team.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "msg": "owners successfully removed",
      "ok": true,
    }
  `);

  let owners = db.crateOwnership.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(owners.length).toBe(1);
  expect(owners[0].user.id).toBe(user.id);
  expect(owners[0].team).toBe(null);

  let invites = db.crateOwnerInvitation.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(invites.length).toBe(0);
});

test('can remove multiple owners', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let crate = await db.crate.create({ name: 'foo' });
  await db.crateOwnership.create({ crate, user });

  let team = await db.team.create();
  await db.crateOwnership.create({ crate, team });

  let user2 = await db.user.create();
  await db.crateOwnership.create({ crate, user: user2 });

  let body = JSON.stringify({ owners: [user2.login, team.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "msg": "owners successfully removed",
      "ok": true,
    }
  `);

  let owners = db.crateOwnership.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(owners.length).toBe(1);
  expect(owners[0].user.id).toBe(user.id);
  expect(owners[0].team).toBe(null);

  let invites = db.crateOwnerInvitation.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(invites.length).toBe(0);
});
