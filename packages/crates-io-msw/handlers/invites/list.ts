import { db } from '../../index.js';
import { serializeInvite } from '../../serializers/invite.js';
import { serializeUser } from '../../serializers/user.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/crate_owner_invitations', ({ request, response }) => {
  let url = new URL(request.url);
  let crateName = url.searchParams.get('crate_name');
  let inviteeId = url.searchParams.get('invitee_id');
  if (crateName === null && inviteeId === null) {
    return response('4XX').json({ errors: [{ detail: 'missing or invalid filter' }] }, { status: 400 });
  }

  let { user } = getSession();
  if (!user) {
    return response('4XX').json({ errors: [{ detail: 'this action requires authentication' }] }, { status: 403 });
  }

  let invites;
  if (crateName) {
    let crate = db.crate.findFirst(q => q.where({ name: crateName }));
    if (!crate) return response('4XX').json(notFoundError(), { status: 404 });

    let isOwner = db.crateOwnership.findFirst(q =>
      q.where(ownership => ownership.crate.id === crate.id && ownership.user?.id === user.id),
    );
    if (!isOwner) {
      let detail = 'only crate owners can query pending invitations for their crate';
      return response('4XX').json({ errors: [{ detail }] }, { status: 403 });
    }

    invites = db.crateOwnerInvitation.findMany(q => q.where(invite => invite.crate.id === crate.id));
  } else {
    if (Number(inviteeId) !== user.id) {
      let detail = 'only the invitee can query their pending invitations';
      return response('4XX').json({ errors: [{ detail }] }, { status: 403 });
    }

    invites = db.crateOwnerInvitation.findMany(q => q.where(invite => invite.invitee.id === user.id));
  }

  let now = Date.now();
  invites = invites
    .filter(invite => Date.parse(invite.expiresAt) > now)
    .toSorted((a, b) => a.crate.id - b.crate.id || a.invitee.id - b.invitee.id);

  let seek = url.searchParams.get('seek');
  if (seek) {
    let [crateId, inviteeId] = JSON.parse(atob(seek));
    invites = invites.filter(
      invite => invite.crate.id > crateId || (invite.crate.id === crateId && invite.invitee.id > inviteeId),
    );
  }

  let perPage = Number(url.searchParams.get('per_page') ?? 10);
  let nextPage = null;
  if (invites.length > perPage) {
    invites = invites.slice(0, perPage);
    let last = invites.at(-1)!;
    url.searchParams.set('seek', btoa(JSON.stringify([last.crate.id, last.invitee.id])));
    nextPage = url.search;
  }

  let users = [...new Set(invites.flatMap(invite => [invite.inviter, invite.invitee]))];

  return response(200).json({
    crate_owner_invitations: invites.map(invite => serializeInvite(invite)),
    users: users.toSorted((a, b) => a.id - b.id).map(user => serializeUser(user)),
    meta: { next_page: nextPage },
  });
});
