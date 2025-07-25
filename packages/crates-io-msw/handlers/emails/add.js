import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeEmail } from '../../serializers/email.js';
import { getSession } from '../../utils/session.js';

export default http.post('/api/v1/users/:user_id/emails', async ({ params, request }) => {
  let { user_id } = params;

  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }
  if (user.id.toString() !== user_id) {
    return HttpResponse.json({ errors: [{ detail: 'current user does not match requested user' }] }, { status: 400 });
  }

  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'User not found.' }] }, { status: 404 });
  }

  let email = db.email.create({
    email: (await request.json()).email,
    verified: false,
    verification_email_sent: true,
    send_notifications: false,
  });
  db.user.update({
    where: { id: { equals: user.id } },
    data: {
      emails: [...user.emails, email],
    },
  });

  return HttpResponse.json(serializeEmail(email));
});
