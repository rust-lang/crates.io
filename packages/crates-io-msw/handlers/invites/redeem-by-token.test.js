import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('can accept an invitation', async function () {
  let serde = await db.crate.create({ name: 'serde' });
  await db.version.create({ crate: serde });

  let inviter = await db.user.create();
  let invitee = await db.user.create();
  await db.mswSession.create({ user: invitee });

  let invite = await db.crateOwnerInvitation.create({ crate: serde, invitee, inviter });

  let response = await fetch(`/api/v1/me/crate_owner_invitations/accept/${invite.token}`, { method: 'PUT' });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "crate_owner_invitation": {
        "accepted": true,
        "crate_id": 1,
      },
    }
  `);

  let invites = db.crateOwnerInvitation.findMany(q =>
    q.where({ crate: { id: serde.id }, invitee: { id: invitee.id } }),
  );
  expect(invites.length).toBe(0);
  let owners = db.crateOwnership.findMany(q => q.where({ crate: { id: serde.id }, user: { id: invitee.id } }));
  expect(owners.length).toBe(1);
});

test('returns 404 if invite does not exist', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/me/crate_owner_invitations/accept/secret-token', { method: 'PUT' });
  expect(response.status).toBe(404);
});
