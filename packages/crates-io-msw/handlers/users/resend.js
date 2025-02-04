import { http, HttpResponse } from 'msw';

import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/users/:user_id/resend', ({ params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  if (user.id.toString() !== params.user_id) {
    return HttpResponse.json({ errors: [{ detail: 'current user does not match requested user' }] }, { status: 400 });
  }

  // let's pretend that we're sending an email here... :D

  return HttpResponse.json({ ok: true });
});
