import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/users/:user_id', async ({ params, request }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  if (user.id.toString() !== params.user_id) {
    return HttpResponse.json({ errors: [{ detail: 'current user does not match requested user' }] }, { status: 400 });
  }

  let json = await request.json();
  if (!json || !json.user) {
    return HttpResponse.json({ errors: [{ detail: 'invalid json request' }] }, { status: 400 });
  }

  if (json.user.publish_notifications !== undefined) {
    db.user.update({
      where: { id: { equals: user.id } },
      data: { publishNotifications: json.user.publish_notifications },
    });
  }

  return HttpResponse.json({ ok: true });
});
