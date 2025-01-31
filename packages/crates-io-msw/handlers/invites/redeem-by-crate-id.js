import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/me/crate_owner_invitations/:crate_id', async ({ request }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let body = await request.json();
  let { accepted, crate_id: crateId } = body.crate_owner_invite;

  let invite = db.crateOwnerInvitation.findFirst({
    where: {
      crate: { id: { equals: parseInt(crateId) } },
      invitee: { id: { equals: user.id } },
    },
  });
  if (!invite) return notFound();

  if (accepted) {
    db.crateOwnership.create({ crate: invite.crate, user });
  }

  db.crateOwnerInvitation.delete({ where: { id: invite.id } });

  return HttpResponse.json({ crate_owner_invitation: { crate_id: crateId, accepted } });
});
