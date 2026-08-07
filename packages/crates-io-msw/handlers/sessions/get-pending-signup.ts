import { db } from '../../index.js';
import { serializePendingSignup } from '../../serializers/pending-signup.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/private/session/signup', ({ response }) => {
  let pendingSignup = db.pendingSignup.findFirst();
  if (!pendingSignup) {
    let detail = 'Your signup session is missing or has expired. Please authenticate with GitHub again.';
    return response('4XX').json({ errors: [{ detail }] }, { status: 400 });
  }

  let signup = serializePendingSignup(pendingSignup);
  return response(200).json({ signup });
});
