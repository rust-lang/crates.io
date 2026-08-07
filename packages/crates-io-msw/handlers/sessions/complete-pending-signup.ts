import * as v from 'valibot';

import { db } from '../../index.js';
import { serializeUser } from '../../serializers/user.js';
import { http } from '../../utils/openapi-http.js';

const EMAIL_SCHEMA = v.pipe(v.string(), v.email());

export default http.post('/api/private/session/signup', async ({ request, response }) => {
  let pendingSignup = db.pendingSignup.findFirst();
  if (!pendingSignup) {
    let detail = 'Your signup session is missing or has expired. Please authenticate with GitHub again.';
    return response('4XX').json({ errors: [{ detail }] }, { status: 400 });
  }

  let body = await request.json();
  let email = body.signup?.email;
  if (!email) {
    let detail = 'Please enter an email address.';
    return response('4XX').json({ errors: [{ detail }] }, { status: 422 });
  }
  if (!v.safeParse(EMAIL_SCHEMA, email).success) {
    let detail = 'Please enter a valid email address.';
    return response('4XX').json({ errors: [{ detail }] }, { status: 422 });
  }

  let user = await db.user.create({
    login: pendingSignup.login,
    email,
    emailVerified: false,
    emailVerificationToken: 'pending-signup',
  });
  await db.mswSession.deleteMany(q => q.where(() => true));
  await db.mswSession.create({ user });
  await db.pendingSignup.deleteMany(q => q.where(() => true));

  return response(200).json({
    user: serializeUser(user, { removePrivateData: false }),
    owned_crates: [],
  });
});
