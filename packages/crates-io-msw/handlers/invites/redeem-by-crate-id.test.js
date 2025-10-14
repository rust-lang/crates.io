import { test as _test, assert } from 'vitest';

import { db } from '../../index.js';

let test = _test.extend({
  // eslint-disable-next-line no-empty-pattern
  serde: async ({}, use) => {
    let serde = await db.crate.create({ name: 'serde' });
    await db.version.create({ crate: serde });
    await use(serde);
  },
});

test('can accept an invitation', async function ({ serde }) {
  let inviter = await db.user.create();
  let invitee = await db.user.create();
  await db.mswSession.create({ user: invitee });

  await db.crateOwnerInvitation.create({ crate: serde, invitee, inviter });

  let body = JSON.stringify({ crate_owner_invite: { crate_id: serde.id, accepted: true } });
  let response = await fetch('/api/v1/me/crate_owner_invitations/serde', { method: 'PUT', body });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    crate_owner_invitation: {
      accepted: true,
      crate_id: serde.id,
    },
  });

  let invites = db.crateOwnerInvitation.findMany(q =>
    q.where({ crate: { id: serde.id }, invitee: { id: invitee.id } }),
  );
  assert.strictEqual(invites.length, 0);
  let owners = db.crateOwnership.findMany(q => q.where({ crate: { id: serde.id }, user: { id: invitee.id } }));
  assert.strictEqual(owners.length, 1);
});

test('can decline an invitation', async function ({ serde }) {
  let inviter = await db.user.create();
  let invitee = await db.user.create();
  await db.mswSession.create({ user: invitee });

  await db.crateOwnerInvitation.create({ crate: serde, invitee, inviter });

  let body = JSON.stringify({ crate_owner_invite: { crate_id: serde.id, accepted: false } });
  let response = await fetch('/api/v1/me/crate_owner_invitations/serde', { method: 'PUT', body });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    crate_owner_invitation: {
      accepted: false,
      crate_id: serde.id,
    },
  });

  let invites = db.crateOwnerInvitation.findMany(q =>
    q.where({ crate: { id: serde.id }, invitee: { id: invitee.id } }),
  );
  assert.strictEqual(invites.length, 0);
  let owners = db.crateOwnership.findMany(q => q.where({ crate: { id: serde.id }, user: { id: invitee.id } }));
  assert.strictEqual(owners.length, 0);
});

test('returns 404 if invite does not exist', async function ({ serde }) {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let body = JSON.stringify({ crate_owner_invite: { crate_id: serde.id, accepted: true } });
  let response = await fetch('/api/v1/me/crate_owner_invitations/serde', { method: 'PUT', body });
  assert.strictEqual(response.status, 404);
});

test('returns 404 if invite is for another user', async function ({ serde }) {
  let inviter = await db.user.create();
  let invitee = await db.user.create();
  await db.mswSession.create({ user: inviter });

  await db.crateOwnerInvitation.create({ crate: serde, invitee, inviter });

  let body = JSON.stringify({ crate_owner_invite: { crate_id: serde.id, accepted: true } });
  let response = await fetch('/api/v1/me/crate_owner_invitations/serde', { method: 'PUT', body });
  assert.strictEqual(response.status, 404);
});

test('returns an error if unauthenticated', async function ({ serde }) {
  let body = JSON.stringify({ crate_owner_invite: { crate_id: serde.id, accepted: true } });
  let response = await fetch('/api/v1/me/crate_owner_invitations/serde', { method: 'PUT', body });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
