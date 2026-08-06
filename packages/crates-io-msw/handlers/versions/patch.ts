import { db } from '../../index.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.patch('/api/v1/crates/{name}/{version}', async ({ request, params, response }) => {
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

  let body = await request.json();

  let yanked = body.version.yanked ?? version.yanked;
  let yankMessage = body.version.yank_message;

  let versionId = version.id;
  version = await db.version.update(q => q.where({ id: versionId }), {
    data(version) {
      version.yanked = yanked;
      version.yank_message = yanked ? yankMessage || null : null;
    },
  });

  return response(200).json({ version: serializeVersion(version!) });
});
