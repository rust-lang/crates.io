import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeDependency } from '../../serializers/dependency.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/crates/:name/:version/dependencies', async ({ params }) => {
  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) return notFound();

  let version = db.version.findFirst({
    where: {
      crate: { id: { equals: crate.id } },
      num: { equals: params.version },
    },
  });
  if (!version) {
    let errorMessage = `crate \`${crate.name}\` does not have a version \`${params.version}\``;
    return HttpResponse.json({ errors: [{ detail: errorMessage }] }, { status: 404 });
  }

  let dependencies = db.dependency.findMany({ where: { version: { id: { equals: version.id } } } });

  return HttpResponse.json({
    dependencies: dependencies.map(d => serializeDependency(d)),
  });
});
