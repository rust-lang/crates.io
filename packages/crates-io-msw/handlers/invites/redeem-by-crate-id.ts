import { db } from '../../index.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/me/crate_owner_invitations/{crate_id}', async ({ request, response }) => {
  let { user } = getSession();
  if (!user) {
    return response('4XX').json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let body = await request.json();
  let { accepted, crate_id: crateId } = body.crate_owner_invite;

  let invite = db.crateOwnerInvitation.findFirst(q =>
    q.where(invite => invite.crate.id === crateId && invite.invitee.id === user.id),
  );
  if (!invite) return response('4XX').json(notFoundError(), { status: 404 });

  if (accepted) {
    await db.crateOwnership.create({ crate: invite.crate, user });
  }

  db.crateOwnerInvitation.delete(q => q.where({ id: invite.id }));

  return response(200).json({
    crate_owner_invitation: { crate_id: invite.crate.id, accepted },
  });
});
