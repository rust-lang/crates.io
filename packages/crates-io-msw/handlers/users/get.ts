import { db } from '../../index.js';
import { serializeUser } from '../../serializers/user.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

function canonUsername(username: string) {
  return username.toLowerCase().replaceAll('-', '_');
}

export default http.get('/api/v1/users/{user}', ({ params, response }) => {
  let name = canonUsername(params.user);
  let user = db.user.findMany(q => q.where(user => canonUsername(user.login) === name), {
    orderBy: { id: 'desc' },
  })[0];
  if (!user) {
    return response('4XX').json(notFoundError(), { status: 404 });
  }

  return response(200).json({ user: serializeUser(user) });
});
