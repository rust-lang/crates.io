import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeCrate } from '../../serializers/crate.js';
import { getSession } from '../../utils/session.js';

export default http.patch<{ name: string }>('/api/v1/crates/:name', async ({ request, params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return HttpResponse.json({ errors: [{ detail: `crate \`${params.name}\` does not exist` }] }, { status: 404 });
  }

  let body = (await request.json()) as { crate?: { trustpub_only?: boolean } };

  if (body.crate?.trustpub_only != null) {
    let trustpubOnly = body.crate.trustpub_only;
    let crateId = crate.id;
    crate = await db.crate.update(q => q.where({ id: crateId }), {
      data(crate) {
        crate.trustpubOnly = trustpubOnly;
      },
    });
  }

  return HttpResponse.json({ crate: serializeCrate(crate!) });
});
