import { assert, test } from 'vitest';

import { db } from '../../index.js';

const REMOVE_USER_BODY = JSON.stringify({ owners: ['john-doe'] });

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body: REMOVE_USER_BODY });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body: REMOVE_USER_BODY });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('can remove a user owner', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let crate = db.crate.create({ name: 'foo' });
  db.crateOwnership.create({ crate, user });

  let user2 = db.user.create();
  db.crateOwnership.create({ crate, user: user2 });

  let body = JSON.stringify({ owners: [user2.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true, msg: 'owners successfully removed' });

  let owners = db.crateOwnership.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(owners.length, 1);
  assert.strictEqual(owners[0].user.id, user.id);

  let invites = db.crateOwnerInvitation.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(invites.length, 0);
});

test('can remove a team owner', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let crate = db.crate.create({ name: 'foo' });
  db.crateOwnership.create({ crate, user });

  let team = db.team.create();
  db.crateOwnership.create({ crate, team });

  let body = JSON.stringify({ owners: [team.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true, msg: 'owners successfully removed' });

  let owners = db.crateOwnership.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(owners.length, 1);
  assert.strictEqual(owners[0].user.id, user.id);
  assert.strictEqual(owners[0].team, null);

  let invites = db.crateOwnerInvitation.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(invites.length, 0);
});

test('can remove multiple owners', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let crate = db.crate.create({ name: 'foo' });
  db.crateOwnership.create({ crate, user });

  let team = db.team.create();
  db.crateOwnership.create({ crate, team });

  let user2 = db.user.create();
  db.crateOwnership.create({ crate, user: user2 });

  let body = JSON.stringify({ owners: [user2.login, team.login] });
  let response = await fetch('/api/v1/crates/foo/owners', { method: 'DELETE', body });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true, msg: 'owners successfully removed' });

  let owners = db.crateOwnership.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(owners.length, 1);
  assert.strictEqual(owners[0].user.id, user.id);
  assert.strictEqual(owners[0].team, null);

  let invites = db.crateOwnerInvitation.findMany({ where: { crate: { id: { equals: crate.id } } } });
  assert.strictEqual(invites.length, 0);
});
