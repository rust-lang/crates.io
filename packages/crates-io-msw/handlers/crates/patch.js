import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeCrate } from '../../serializers/crate.js';
import { getSession } from '../../utils/session.js';

export default http.patch('/api/v1/crates/:name', async ({ request, params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return HttpResponse.json({ errors: [{ detail: `crate \`${params.name}\` does not exist` }] }, { status: 404 });
  }

  let body = await request.json();

  if (body.crate?.trustpub_only != null) {
    crate = await db.crate.update(q => q.where({ id: crate.id }), {
      data(crate) {
        crate.trustpubOnly = body.crate.trustpub_only;
      },
    });
  }

  return HttpResponse.json({ crate: serializeCrate(crate) });
});
