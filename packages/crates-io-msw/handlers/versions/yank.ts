import { db } from '../../index.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.delete('/api/v1/crates/{name}/{version}/yank', async ({ params, response }) => {
  let { user } = getSession();
  if (!user) {
    return response('4XX').json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) return response('4XX').json(notFoundError(), { status: 404 });

  let version = db.version.findFirst(q =>
    q.where(version => version.crate.id === crate.id && version.num === params.version),
  );
  if (!version) return response('4XX').json(notFoundError(), { status: 404 });

  await db.version.update(q => q.where({ id: version.id }), {
    data(version) {
      version.yanked = true;
    },
  });

  return response(200).json({ ok: true });
});
