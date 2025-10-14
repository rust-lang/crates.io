import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';

export default http.put('/api/v1/confirm/:token', async ({ params }) => {
  let { token } = params;

  let user = db.user.findFirst(q => q.where({ emailVerificationToken: token }));
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'Email belonging to token not found.' }] }, { status: 400 });
  }

  await db.user.update(q => q.where({ id: user.id }), {
    data(user) {
      user.emailVerified = true;
      user.emailVerificationToken = null;
    },
  });

  return HttpResponse.json({ ok: true });
});
