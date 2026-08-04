import { db } from '../../index.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.delete('/api/v1/crates/{name}/owners', async ({ request, params, response }) => {
  let { user } = getSession();
  if (!user) {
    return response('4XX').json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return response('4XX').json(notFoundError(), { status: 404 });
  }

  let body = await request.json();

  for (let owner of body.owners) {
    let ownership = db.crateOwnership.findFirst(
      owner.includes(':')
        ? q => q.where(ownership => ownership.team?.login === owner)
        : q => q.where(ownership => ownership.user?.login === owner),
    );
    if (!ownership) return response('4XX').json(notFoundError(), { status: 404 });
    db.crateOwnership.delete(q => q.where({ id: ownership.id }));
  }

  return response(200).json({ ok: true, msg: 'owners successfully removed' });
});
