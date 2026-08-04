import { db } from '../../index.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/crates/{name}/downloads', ({ params, query, response }) => {
  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) return response('4XX').json(notFoundError(), { status: 404 });

  let downloads = db.versionDownload.findMany(q => q.where(download => download.version.crate.id === crate.id));
  let include = query.get('include') ?? '';
  let includes = include ? include.split(',') : [];
  let versions = includes.includes('versions') ? [...new Set(downloads.map(it => it.version))] : undefined;

  return response(200).json({
    version_downloads: downloads.map(download => ({
      date: download.date,
      downloads: download.downloads,
      version: download.version.id,
    })),
    meta: { extra_downloads: crate._extra_downloads },
    ...(versions && { versions: versions.map(it => serializeVersion(it)) }),
  });
});
