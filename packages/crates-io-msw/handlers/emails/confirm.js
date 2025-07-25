import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeEmail } from '../../serializers/email.js';

export default http.put('/api/v1/confirm/:token', ({ params }) => {
  let { token } = params;

  let email = db.email.findFirst({ where: { token: { equals: token } } });
  if (!email) {
    return HttpResponse.json({ errors: [{ detail: 'Email belonging to token not found.' }] }, { status: 400 });
  }

  db.email.update({ where: { id: email.id }, data: { verified: true } });

  return HttpResponse.json({
    ok: true,
    email: serializeEmail({
      ...email,
      verified: true,
    }),
  });
});
