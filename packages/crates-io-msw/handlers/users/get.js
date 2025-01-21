import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeUser } from '../../serializers/user.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/users/:user_id', ({ params }) => {
  let login = params.user_id;
  let user = db.user.findFirst({ where: { login: { equals: login } } });
  if (!user) {
    return notFound();
  }

  return HttpResponse.json({ user: serializeUser(user) });
});
