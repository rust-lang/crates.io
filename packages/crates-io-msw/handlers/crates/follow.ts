import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/crates/{name}/follow', async ({ params, response }) => {
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

  await db.user.update(q => q.where({ id: user.id }), {
    data(user) {
      user.followedCrates = [...user.followedCrates.filter(c => c.id !== crate.id), crate];
    },
  });

  return response(200).json({ ok: true });
});
