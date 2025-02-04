import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeVersion } from '../../serializers/version.js';
import { pageParams } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/me/updates', ({ request }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let allVersions = user.followedCrates
    .flatMap(crate => db.version.findMany({ where: { crate: { id: { equals: crate.id } } } }))
    .sort((a, b) => b.id - a.id);

  let { start, end, page, perPage } = pageParams(request);

  let versions = allVersions.slice(start, end);
  let totalCount = allVersions.length;
  let totalPages = Math.ceil(totalCount / perPage);

  return HttpResponse.json({
    versions: versions.map(v => serializeVersion(v)),
    meta: { more: page < totalPages },
  });
});
