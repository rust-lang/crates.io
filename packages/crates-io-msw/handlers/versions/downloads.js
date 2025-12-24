import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/crates/:name/:version/downloads', async ({ params }) => {
  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) return notFound();

  let version = db.version.findFirst(q =>
    q.where(version => version.crate.id === crate.id && version.num === params.version),
  );
  if (!version) {
    let errorMessage = `crate \`${crate.name}\` does not have a version \`${params.version}\``;
    return HttpResponse.json({ errors: [{ detail: errorMessage }] }, { status: 404 });
  }

  let downloads = db.versionDownload.findMany(q => q.where(download => download.version.id === version.id));

  return HttpResponse.json({
    version_downloads: downloads.map(download => ({
      date: download.date,
      downloads: download.downloads,
      version: download.version.id,
    })),
  });
});
