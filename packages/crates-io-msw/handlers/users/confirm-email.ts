import { db } from '../../index.js';
import { http } from '../../utils/openapi-http.js';

export default http.put('/api/v1/confirm/{email_token}', async ({ params, response }) => {
  let user = db.user.findFirst(q => q.where({ emailVerificationToken: params.email_token }));
  if (!user) {
    return response('4XX').json({ errors: [{ detail: 'Email belonging to token not found.' }] }, { status: 400 });
  }

  await db.user.update(q => q.where({ id: user.id }), {
    data(user) {
      user.emailVerified = true;
      user.emailVerificationToken = null;
    },
  });

  return response(200).json({ ok: true });
});
