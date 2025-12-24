import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.delete('/api/v1/crates/:name/owners', async ({ request, params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return notFound();
  }

  let body = await request.json();

  for (let owner of body.owners) {
    let ownership = db.crateOwnership.findFirst(
      owner.includes(':')
        ? q => q.where(ownership => ownership.team?.login === owner)
        : q => q.where(ownership => ownership.user?.login === owner),
    );
    if (!ownership) return notFound();
    db.crateOwnership.delete(q => q.where({ id: ownership.id }));
  }

  return HttpResponse.json({ ok: true, msg: 'owners successfully removed' });
});
