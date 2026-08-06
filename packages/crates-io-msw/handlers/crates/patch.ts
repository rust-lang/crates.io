import { db } from '../../index.js';
import { serializeCrate } from '../../serializers/crate.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.patch('/api/v1/crates/{name}', async ({ request, params, response }) => {
  let { user } = getSession();
  if (!user) {
    return response('4XX').json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return response('4XX').json({ errors: [{ detail: `crate \`${params.name}\` does not exist` }] }, { status: 404 });
  }

  let body = await request.json();

  if (body.crate?.trustpub_only != null) {
    let trustpubOnly = body.crate.trustpub_only;
    let crateId = crate.id;
    crate = await db.crate.update(q => q.where({ id: crateId }), {
      data(crate) {
        crate.trustpubOnly = trustpubOnly;
      },
    });
  }

  return response(200).json({ crate: serializeCrate(crate!) });
});
