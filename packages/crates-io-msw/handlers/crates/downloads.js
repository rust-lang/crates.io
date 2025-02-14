import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/crates/:name/downloads', async ({ request, params }) => {
  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) return notFound();

  let downloads = db.versionDownload.findMany({ where: { version: { crate: { id: { equals: crate.id } } } } });
  let resp = {
    version_downloads: downloads.map(download => ({
      date: download.date,
      downloads: download.downloads,
      version: download.version.id,
    })),
    meta: { extra_downloads: crate._extra_downloads },
  };

  let url = new URL(request.url);
  let include = url.searchParams.get('include') ?? '';
  let includes = include ? include.split(',') : [];
  if (includes.includes('versions')) {
    let versions = [...new Set(downloads.map(it => it.version))];
    resp.versions = versions.map(it => serializeVersion(it));
  }
  return HttpResponse.json(resp);
});
