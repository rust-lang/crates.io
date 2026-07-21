import type { SuccessBody } from '../../utils/api-types.js';

import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeUser } from '../../serializers/user.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/me', () => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let ownerships = db.crateOwnership.findMany(q => q.where(ownership => ownership.user?.id === user.id));

  return HttpResponse.json<SuccessBody<'get_authenticated_user'>>({
    user: serializeUser(user, { removePrivateData: false }),
    owned_crates: ownerships.map(ownership => ({
      id: ownership.crate.id,
      name: ownership.crate.name,
      email_notifications: ownership.emailNotifications,
    })),
  });
});
