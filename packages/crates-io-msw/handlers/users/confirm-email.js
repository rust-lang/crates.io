import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';

export default http.put('/api/v1/confirm/:token', ({ params }) => {
  let { token } = params;

  let user = db.user.findFirst({ where: { emailVerificationToken: { equals: token } } });
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'Email belonging to token not found.' }] }, { status: 400 });
  }

  db.user.update({ where: { id: user.id }, data: { emailVerified: true, emailVerificationToken: null } });

  return HttpResponse.json({ ok: true });
});
