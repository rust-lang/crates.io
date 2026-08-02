import { db } from '../../index.js';
import { serializeApiToken } from '../../serializers/api-token.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/me/tokens/{id}', ({ params, response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  let token = db.apiToken.findFirst(q =>
    q.where(token => token.id === parseInt(params.id) && token.user.id === user.id),
  );
  if (!token) return response.untyped(notFound());

  return response(200).json({
    api_token: serializeApiToken(token),
  });
});
