import { db } from '../../index.js';
import { serializeUser } from '../../serializers/user.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/users/{user}', ({ params, response }) => {
  let user = db.user.findFirst(q => q.where({ login: params.user }));
  if (!user) {
    return response.untyped(notFound());
  }

  return response(200).json({ user: serializeUser(user) });
});
