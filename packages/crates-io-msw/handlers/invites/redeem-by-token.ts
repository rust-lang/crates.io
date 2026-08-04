import { db } from '../../index.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.put('/api/v1/me/crate_owner_invitations/accept/{token}', async ({ params, response }) => {
  let { token } = params;

  let invite = db.crateOwnerInvitation.findFirst(q => q.where({ token }));
  if (!invite) return response('4XX').json(notFoundError(), { status: 404 });

  await db.crateOwnership.create({ crate: invite.crate, user: invite.invitee });
  db.crateOwnerInvitation.delete(q => q.where({ id: invite.id }));

  return response(200).json({
    crate_owner_invitation: { crate_id: invite.crate.id, accepted: true },
  });
});
