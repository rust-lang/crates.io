import { db } from '../../index.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/crates/{name}/{version}', ({ params, response }) => {
  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) return response('4XX').json(notFoundError(), { status: 404 });

  let version = db.version.findFirst(q =>
    q.where(version => version.crate.id === crate.id && version.num === params.version),
  );
  if (!version) {
    let errorMessage = `crate \`${crate.name}\` does not have a version \`${params.version}\``;
    return response('4XX').json({ errors: [{ detail: errorMessage }] }, { status: 404 });
  }

  return response(200).json({
    version: serializeVersion(version),
  });
});
