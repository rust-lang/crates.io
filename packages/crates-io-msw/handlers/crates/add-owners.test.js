import { assert, test } from 'vitest';

import { db } from '../../index.js';

const ADD_USER_BODY = JSON.stringify({ owners: ['john-doe'] });

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body: ADD_USER_BODY });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body: ADD_USER_BODY });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('can add new owner', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let crate = db.crate.create({ name: 'foo' });
  db.crateOwnership.create({ crate, user });

  let user2 = db.user.create();

  let body = JSON.stringify({ owners: [user2.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    ok: true,
    msg: 'user user-2 has been invited to be an owner of crate foo',
  });

  let owners = db.crateOwnership.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(owners.length, 1);
  assert.strictEqual(owners[0].user.id, user.id);

  let invites = db.crateOwnerInvitation.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(invites.length, 1);
  assert.strictEqual(invites[0].inviter.id, user.id);
  assert.strictEqual(invites[0].invitee.id, user2.id);
});

test('can add team owner', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let crate = db.crate.create({ name: 'foo' });
  db.crateOwnership.create({ crate, user });

  let team = db.team.create();

  let body = JSON.stringify({ owners: [team.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    ok: true,
    msg: 'team github:rust-lang:team-1 has been added as an owner of crate foo',
  });

  let owners = db.crateOwnership.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(owners.length, 2);
  assert.strictEqual(owners[0].user.id, user.id);
  assert.strictEqual(owners[0].team, null);
  assert.strictEqual(owners[1].user, null);
  assert.strictEqual(owners[1].team.id, user.id);

  let invites = db.crateOwnerInvitation.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(invites.length, 0);
});

test('can add multiple owners', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let crate = db.crate.create({ name: 'foo' });
  db.crateOwnership.create({ crate, user });

  let team = db.team.create();
  let user2 = db.user.create();
  let user3 = db.user.create();

  let body = JSON.stringify({ owners: [user2.login, team.login, user3.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    ok: true,
    msg: 'user user-2 has been invited to be an owner of crate foo,team github:rust-lang:team-1 has been added as an owner of crate foo,user user-3 has been invited to be an owner of crate foo',
  });

  let owners = db.crateOwnership.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(owners.length, 2);
  assert.strictEqual(owners[0].user.id, user.id);
  assert.strictEqual(owners[0].team, null);
  assert.strictEqual(owners[1].user, null);
  assert.strictEqual(owners[1].team.id, user.id);

  let invites = db.crateOwnerInvitation.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(invites.length, 2);
  assert.strictEqual(invites[0].inviter.id, user.id);
  assert.strictEqual(invites[0].invitee.id, user2.id);
  assert.strictEqual(invites[1].inviter.id, user.id);
  assert.strictEqual(invites[1].invitee.id, user3.id);
});
