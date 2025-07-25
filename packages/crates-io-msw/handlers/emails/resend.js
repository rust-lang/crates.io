import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/users/:user_id/emails/:email_id/resend', ({ params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  if (user.id.toString() !== params.user_id) {
    return HttpResponse.json({ errors: [{ detail: 'current user does not match requested user' }] }, { status: 400 });
  }

  let email = db.email.findFirst({ where: { id: { equals: parseInt(params.email_id) } } });
  if (!email) {
    return HttpResponse.json({ errors: [{ detail: 'Email not found.' }] }, { status: 404 });
  }

  // let's pretend that we're sending an email here... :D

  return HttpResponse.json({ ok: true });
});
