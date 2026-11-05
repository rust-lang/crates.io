import { db } from '../../index.js';
import { serializeInvite } from '../../serializers/invite.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.patch('/api/v1/crate_owner_invitations/{id}', async ({ params, request, response }) => {
  let invite = db.crateOwnerInvitation.findFirst(q => q.where({ id: Number(params.id) }));
  if (!invite) return response('4XX').json(notFoundError(), { status: 404 });

  let { user } = getSession();
  if (!user) {
    return response('4XX').json({ errors: [{ detail: 'this action requires authentication' }] }, { status: 403 });
  }

  let body = await request.json();
  let { state } = body.crate_owner_invitation;

  if (invite.invitee.id === user.id) {
    if (state === 'accepted') {
      if (Date.parse(invite.expiresAt) <= Date.now()) {
        let detail = `The invitation to become an owner of the ${invite.crate.name} crate expired. Please reach out to an owner of the crate to request a new invitation.`;
        return response('4XX').json({ errors: [{ detail }] }, { status: 410 });
      }

      await db.crateOwnership.create({ crate: invite.crate, user });
    } else if (state !== 'declined') {
      let detail = '`state` must be `accepted` or `declined`';
      return response('4XX').json({ errors: [{ detail }] }, { status: 400 });
    }
  } else {
    let isOwner = db.crateOwnership.findFirst(q =>
      q.where(ownership => ownership.crate.id === invite.crate.id && ownership.user?.id === user.id),
    );
    if (!isOwner) {
      let detail = 'only the invitee or a crate owner can update this invitation';
      return response('4XX').json({ errors: [{ detail }] }, { status: 403 });
    }

    if (state !== 'revoked') {
      let detail = '`state` must be `revoked`';
      return response('4XX').json({ errors: [{ detail }] }, { status: 400 });
    }
  }

  db.crateOwnerInvitation.delete(q => q.where({ id: invite.id }));

  return response(200).json({ crate_owner_invitation: { ...serializeInvite(invite), state } });
});
