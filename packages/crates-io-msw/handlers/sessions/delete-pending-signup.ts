import { db } from '../../index.js';
import { http } from '../../utils/openapi-http.js';

export default http.delete('/api/private/session/signup', ({ response }) => {
  db.pendingSignup.deleteMany(q => q.where(() => true));
  return response(200).json({ ok: true });
});
