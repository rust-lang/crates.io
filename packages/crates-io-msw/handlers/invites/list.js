import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeInvite } from '../../serializers/invite.js';
import { serializeUser } from '../../serializers/user.js';
import { notFound } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/private/crate_owner_invitations', ({ request }) => {
  let url = new URL(request.url);

  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let invites;
  if (url.searchParams.has('crate_name')) {
    let crate = db.crate.findFirst({ where: { name: { equals: url.searchParams.get('crate_name') } } });
    if (!crate) return notFound();

    invites = db.crateOwnerInvitation.findMany({ where: { crate: { id: { equals: crate.id } } } });
  } else if (url.searchParams.has('invitee_id')) {
    let inviteeId = parseInt(url.searchParams.get('invitee_id'));
    if (inviteeId !== user.id) {
      return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
    }

    invites = db.crateOwnerInvitation.findMany({ where: { invitee: { id: { equals: inviteeId } } } });
  } else {
    return HttpResponse.json({ errors: [{ detail: 'missing or invalid filter' }] }, { status: 400 });
  }

  let perPage = 10;
  let start = parseInt(url.searchParams.get('__start__') ?? '0');
  let end = start + perPage;

  let nextPage = null;
  if (invites.length > end) {
    url.searchParams.set('__start__', end);
    nextPage = url.search;
  }

  invites = invites.slice(start, end);

  let inviters = invites.map(invite => invite.inviter);
  let invitees = invites.map(invite => invite.invitee);
  let users = Array.from(new Set([...inviters, ...invitees])).sort((a, b) => a.id - b.id);

  return HttpResponse.json({
    crate_owner_invitations: invites.map(invite => serializeInvite(invite)),
    users: users.map(user => serializeUser(user)),
    meta: { next_page: nextPage },
  });
});
