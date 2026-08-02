import { db } from '../../index.js';
import { serializeInvite } from '../../serializers/invite.js';
import { serializeUser } from '../../serializers/user.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/private/crate_owner_invitations', ({ request, response }) => {
  let url = new URL(request.url);

  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  let invites;
  if (url.searchParams.has('crate_name')) {
    let crate = db.crate.findFirst(q => q.where({ name: url.searchParams.get('crate_name')! }));
    if (!crate) return response.untyped(notFound());

    invites = db.crateOwnerInvitation.findMany(q => q.where(invite => invite.crate.id === crate.id));
  } else if (url.searchParams.has('invitee_id')) {
    let inviteeId = parseInt(url.searchParams.get('invitee_id')!);
    if (inviteeId !== user.id) {
      return response.untyped(
        Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
      );
    }

    invites = db.crateOwnerInvitation.findMany(q => q.where(invite => invite.invitee.id === inviteeId));
  } else {
    return response.untyped(Response.json({ errors: [{ detail: 'missing or invalid filter' }] }, { status: 400 }));
  }

  let perPage = 10;
  let start = parseInt(url.searchParams.get('__start__') ?? '0');
  let end = start + perPage;

  let nextPage = null;
  if (invites.length > end) {
    url.searchParams.set('__start__', String(end));
    nextPage = url.search;
  }

  invites = invites.slice(start, end);

  let inviters = invites.map(invite => invite.inviter);
  let invitees = invites.map(invite => invite.invitee);
  let users = [...new Set([...inviters, ...invitees])].toSorted((a, b) => a.id - b.id);

  return response(200).json({
    invitations: invites.map(invite => serializeInvite(invite)),
    users: users.map(user => serializeUser(user)),
    meta: { next_page: nextPage },
  });
});
