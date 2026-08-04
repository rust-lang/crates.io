import type { Crate } from '../../models/index.js';

import { db } from '../../index.js';
import { serializeVersion } from '../../serializers/version.js';
import { pageParams } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/me/updates', ({ request, response }) => {
  let { user } = getSession();
  if (!user) {
    return response('4XX').json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let allVersions = (user.followedCrates as Crate[])
    .flatMap(crate => db.version.findMany(q => q.where(version => version.crate.id === crate.id)))
    .toSorted((a, b) => b.id - a.id);

  let { start, end, page, perPage } = pageParams(request);

  let versions = allVersions.slice(start, end);
  let totalCount = allVersions.length;
  let totalPages = Math.ceil(totalCount / perPage);

  return response(200).json({
    versions: versions.map(v => serializeVersion(v)),
    meta: { more: page < totalPages },
  });
});
