import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/users/:user_id/emails/:email_id/set_primary', async ({ params }) => {
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

  // Update email to set as primary
  db.email.update({
    where: { id: { equals: parseInt(email_id) } },
    data: { primary: true },
  });
  // Update all other emails to remove primary status
  db.email.updateMany({
    where: { user_id: { equals: user.id }, id: { notEquals: parseInt(email_id) } },
    data: { primary: false },
  });

  let updatedEmail = db.email.findFirst({ where: { id: { equals: parseInt(email_id) } } });

  return HttpResponse.json(updatedEmail);
});
