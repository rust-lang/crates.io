import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/users/{id}/resend', ({ params, response }) => {
  let { user } = getSession();
  if (!user) {
    return response('4XX').json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  if (user.id.toString() !== params.id) {
    return response('4XX').json(
      { errors: [{ detail: 'current user does not match requested user' }] },
      { status: 400 },
    );
  }

  // let's pretend that we're sending an email here... :D

  return response(200).json({ ok: true });
});
