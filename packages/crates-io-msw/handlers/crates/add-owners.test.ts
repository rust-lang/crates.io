import { expect, test } from 'vitest';

import { db } from '../../index.js';

const ADD_USER_BODY = JSON.stringify({ owners: ['john-doe'] });

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body: ADD_USER_BODY });
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
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body: ADD_USER_BODY });
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

test('can add new owner', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let crate = await db.crate.create({ name: 'foo' });
  await db.crateOwnership.create({ crate, user });

  let user2 = await db.user.create({});

  let body = JSON.stringify({ owners: [user2.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "msg": "user user-2 has been invited to be an owner of crate foo",
      "ok": true,
    }
  `);

  let owners = db.crateOwnership.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(owners.length).toBe(1);
  expect(owners[0].user.id).toBe(user.id);

  let invites = db.crateOwnerInvitation.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(invites.length).toBe(1);
  expect(invites[0].inviter.id).toBe(user.id);
  expect(invites[0].invitee.id).toBe(user2.id);
});

test('can add team owner', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let crate = await db.crate.create({ name: 'foo' });
  await db.crateOwnership.create({ crate, user });

  let team = await db.team.create({});

  let body = JSON.stringify({ owners: [team.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "msg": "team github:rust-lang:team-1 has been added as an owner of crate foo",
      "ok": true,
    }
  `);

  let owners = db.crateOwnership.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(owners.length).toBe(2);
  expect(owners[0].user.id).toBe(user.id);
  expect(owners[0].team).toBe(null);
  expect(owners[1].user).toBe(null);
  expect(owners[1].team.id).toBe(user.id);

  let invites = db.crateOwnerInvitation.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(invites.length).toBe(0);
});

test('can add multiple owners', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let crate = await db.crate.create({ name: 'foo' });
  await db.crateOwnership.create({ crate, user });

  let team = await db.team.create({});
  let user2 = await db.user.create({});
  let user3 = await db.user.create({});

  let body = JSON.stringify({ owners: [user2.login, team.login, user3.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "msg": "user user-2 has been invited to be an owner of crate foo,team github:rust-lang:team-1 has been added as an owner of crate foo,user user-3 has been invited to be an owner of crate foo",
      "ok": true,
    }
  `);

  let owners = db.crateOwnership.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(owners.length).toBe(2);
  expect(owners[0].user.id).toBe(user.id);
  expect(owners[0].team).toBe(null);
  expect(owners[1].user).toBe(null);
  expect(owners[1].team.id).toBe(user.id);

  let invites = db.crateOwnerInvitation.findMany(q => q.where({ crate: { id: crate.id } }));
  expect(invites.length).toBe(2);
  expect(invites[0].inviter.id).toBe(user.id);
  expect(invites[0].invitee.id).toBe(user2.id);
  expect(invites[1].inviter.id).toBe(user.id);
  expect(invites[1].invitee.id).toBe(user3.id);
});
