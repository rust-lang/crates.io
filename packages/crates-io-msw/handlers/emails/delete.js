import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { getSession } from '../../utils/session.js';

export default http.delete('/api/v1/users/:user_id/emails/:email_id', ({ params }) => {
  let { user_id, email_id } = params;

  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }
  if (user.id.toString() !== user_id) {
    return HttpResponse.json({ errors: [{ detail: 'current user does not match requested user' }] }, { status: 400 });
  }

  let email = db.email.findFirst({ where: { id: { equals: parseInt(email_id) } } });
  if (!email) {
    return HttpResponse.json({ errors: [{ detail: 'Email not found.' }] }, { status: 404 });
  }

  // Prevent deletion if the email has notifications enabled
  if (email.send_notifications) {
    return HttpResponse.json(
      { errors: [{ detail: 'Cannot delete an email that has notifications enabled.' }] },
      { status: 400 },
    );
  }

  // Check how many emails the user has, if this is the only verified email, prevent deletion
  let userEmails = db.email.findMany({ where: { user_id: { equals: user.id } } });
  if (userEmails.length === 1) {
    return HttpResponse.json({ errors: [{ detail: 'Cannot delete your only email address.' }] }, { status: 400 });
  }

  db.email.delete({ where: { id: { equals: parseInt(email_id) } } });

  return HttpResponse.json({ ok: true });
});
