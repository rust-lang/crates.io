import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';

export default http.put('/api/v1/me/crate_owner_invitations/accept/:token', async ({ params }) => {
  let { token } = params;

  let invite = db.crateOwnerInvitation.findFirst({ where: { token: { equals: token } } });
  if (!invite) return notFound();

  db.crateOwnership.create({ crate: invite.crate, user: invite.invitee });
  db.crateOwnerInvitation.delete({ where: { id: invite.id } });

  return HttpResponse.json({ crate_owner_invitation: { crate_id: invite.crate.id, accepted: true } });
});
