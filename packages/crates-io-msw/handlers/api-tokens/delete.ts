import { db } from '../../index.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.delete('/api/v1/me/tokens/{id}', ({ params, response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  db.apiToken.delete(q => q.where(token => token.id === parseInt(params.id) && token.user.id === user.id));

  return response(200).json({});
});
