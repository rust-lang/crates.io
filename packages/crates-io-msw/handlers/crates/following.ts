import type { Crate } from '../../models/index.js';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/crates/{name}/following', ({ params, response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return response.untyped(notFound());
  }

  let following = (user.followedCrates as Crate[]).some(followedCrate => followedCrate.id === crate.id);

  return response(200).json({ following });
});
