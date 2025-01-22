import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeInvite } from '../../serializers/invite.js';
import { serializeUser } from '../../serializers/user.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/me/crate_owner_invitations', () => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let invites = db.crateOwnerInvitation.findMany({ where: { invitee: { id: { equals: user.id } } } });

  let inviters = invites.map(invite => invite.inviter);
  let invitees = invites.map(invite => invite.invitee);
  let users = Array.from(new Set([...inviters, ...invitees])).sort((a, b) => a.id - b.id);

  return HttpResponse.json({
    crate_owner_invitations: invites.map(invite => serializeInvite(invite)),
    users: users.map(user => serializeUser(user)),
  });
});
