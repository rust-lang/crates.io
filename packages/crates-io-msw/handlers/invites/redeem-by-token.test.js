import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('can accept an invitation', async function () {
  let serde = await db.crate.create({ name: 'serde' });
  await db.version.create({ crate: serde });

  let inviter = await db.user.create();
  let invitee = await db.user.create();
  await db.mswSession.create({ user: invitee });

  let invite = await db.crateOwnerInvitation.create({ crate: serde, invitee, inviter });

  let response = await fetch(`/api/v1/me/crate_owner_invitations/accept/${invite.token}`, { method: 'PUT' });
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

test('returns 404 if invite does not exist', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/me/crate_owner_invitations/accept/secret-token', { method: 'PUT' });
  assert.strictEqual(response.status, 404);
});
