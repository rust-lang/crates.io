import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/crates/:name/downloads', async ({ params }) => {
  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) return notFound();

  let downloads = db.versionDownload.findMany({ where: { version: { crate: { id: { equals: crate.id } } } } });

  return HttpResponse.json({
    version_downloads: downloads.map(download => ({
      date: download.date,
      downloads: download.downloads,
      version: download.version.id,
    })),
    meta: { extra_downloads: crate._extra_downloads },
  });
});
