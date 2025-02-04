import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeUser } from '../../serializers/user.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/crates/:name/owner_user', async ({ params }) => {
  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) {
    return notFound();
  }

  let ownerships = db.crateOwnership.findMany({ where: { crate: { id: { equals: crate.id } } } });

  return HttpResponse.json({
    users: ownerships.filter(o => o.user).map(o => ({ ...serializeUser(o.user), kind: 'user' })),
  });
});
