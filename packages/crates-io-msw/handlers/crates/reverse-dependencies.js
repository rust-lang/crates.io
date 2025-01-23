import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeDependency } from '../../serializers/dependency.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFound, pageParams } from '../../utils/handlers.js';

export default http.get('/api/v1/crates/:name/reverse_dependencies', async ({ request, params }) => {
  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) return notFound();

  let { start, end } = pageParams(request);

  let allDependencies = db.dependency.findMany({
    where: { crate: { id: { equals: crate.id } } },
    orderBy: { version: { crate: { downloads: 'desc' } } },
  });

  let dependencies = allDependencies.slice(start, end);
  let total = allDependencies.length;

  let versions = dependencies.map(d => d.version);

  return HttpResponse.json({
    dependencies: dependencies.map(d => serializeDependency(d)),
    versions: versions.map(v => serializeVersion(v)),
    meta: { total },
  });
});
