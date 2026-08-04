import { db } from '../../index.js';
import { serializeDependency } from '../../serializers/dependency.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFoundError, pageParams } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/crates/{name}/reverse_dependencies', ({ request, params, response }) => {
  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) return response('4XX').json(notFoundError(), { status: 404 });

  let { start, end } = pageParams(request);

  let allDependencies = db.dependency.findMany(q => q.where(dep => dep.crate.id === crate.id), {
    orderBy: { version: { crate: { downloads: 'desc' } } },
  });

  let dependencies = allDependencies.slice(start, end);
  let total = allDependencies.length;

  let versions = dependencies.map(d => d.version);

  return response(200).json({
    dependencies: dependencies.map(d => serializeDependency(d)),
    versions: versions.map(v => serializeVersion(v)),
    meta: { total },
  });
});
